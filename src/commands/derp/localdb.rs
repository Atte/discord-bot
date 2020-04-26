use super::{Image, ImageResponse, RepresentationList};
use crate::CONFIG;
use log::trace;
use postgres::{Client, types::ToSql};
use std::convert::TryFrom;

error_chain::error_chain! {
    foreign_links {
        Postgres(postgres::Error);
    }

    errors {
        NotEnabled {
            description("postgres fallback not enabled")
        }
    }
}

fn connect() -> Result<Client> {
    if let Some(ref config) = CONFIG.gib.postgres {
        Ok(Client::configure()
            .application_name("flutterbitch")
            .user(config.user.as_ref())
            .dbname(config.dbname.as_ref())
            .host(config.host.as_ref())
            .connect(postgres::NoTls)?)
    } else {
        Err(ErrorKind::NotEnabled.into())
    }
}

fn resolve_tag_aliases(client: &mut Client, tags: &(dyn ToSql + Sync)) -> Result<Vec<String>> {
    Ok(client.query(
        r###"
        SELECT COALESCE(tags.name, inputs.name) AS name
        FROM (SELECT unnest($1::text[]) AS name) AS inputs
        LEFT JOIN tags AS input_tags ON input_tags.name = inputs.name
        LEFT JOIN tag_aliases ON tag_aliases.tag_id = input_tags.id
        LEFT JOIN tags ON tag_aliases.target_tag_id = tags.id
        "###,
        &[tags]
    )?.into_iter().map(|row| row.get("name")).collect())
}

pub fn query(search: &str) -> Result<ImageResponse> {
    let (negative_terms, positive_terms): (Vec<String>, Vec<String>) = search
        .split(',')
        .map(|term| term.trim().replace('+', " "))
        .filter(|term| !term.is_empty() && term != "*")
        .partition(|term| term.starts_with('!') || term.starts_with('-'));

    let negative_terms: Vec<&str> = negative_terms
        .iter()
        .filter_map(|term| term.get(1..))
        .collect();

    let mut client = connect()?;
    let positive_terms = resolve_tag_aliases(&mut client, &positive_terms)?;
    let negative_terms = resolve_tag_aliases(&mut client, &negative_terms)?;

    trace!(
        "Searching local db for {:?} not {:?}...",
        positive_terms,
        negative_terms
    );

    let result = client.query(
        r###"
            SELECT *, COUNT(*) OVER() AS total
            FROM flutterbitch
            WHERE tags @> $1::text[] AND NOT (tags && $2::text[])
            ORDER BY random()
            LIMIT 50;
        "###,
        &[&positive_terms, &negative_terms],
    )?;

    let mut result_iter = result.into_iter().peekable();
    let total: i64 = result_iter
        .peek()
        .map_or(0, |row| row.try_get("total").unwrap_or_default());
    let images: Vec<Image> = result_iter
        .map(|row| Image {
            id: row.get("id"),
            tags: row.try_get("tags").unwrap_or_default(),
            description: String::new(),
            name: row.try_get("name").unwrap_or_default(),
            first_seen_at: row.try_get("time").unwrap_or_default(),
            representations: RepresentationList {
                tall: format!(
                    "{}tall.{}",
                    row.get::<_, &str>("path"),
                    row.get::<_, &str>("filext")
                ),
            },
        })
        .collect();

    Ok(ImageResponse {
        total: usize::try_from(total).unwrap_or_default(),
        images,
    })
}
