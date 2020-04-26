use super::{Image, ImageResponse, RepresentationList};
use crate::CONFIG;
use postgres::Client;
use log::trace;

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

pub fn query(search: &str) -> Result<ImageResponse> {
    trace!("Searching local db...");

    let terms: Vec<&str> = search.split(',').map(str::trim).collect();
    let blacklist = &CONFIG.gib.postgres.as_ref().unwrap().filter;

    let mut client = connect()?;
    let images: Vec<Image> = client
        .query(
            r###"
            SELECT
                images.id,
                MIN(images.version_path) AS path,
                MIN(images.image_name) AS name,
                to_char(MIN(images.created_at), 'YYYY-MM-DD"T"HH24:MI:SSOF') AS time,
                array_agg(artist_tags.name) AS tags
            FROM
                images,
                image_taggings,
                (
                    SELECT tags.id
                    FROM tags
                    WHERE tags.name = ANY($1::text[])
                ) AS good_tags,
                (
                    SELECT tags.id
                    FROM tags
                    WHERE tags.name = ANY($2::text[])
                ) AS bad_tags,
                (
                    SELECT tags.id, tags.name
                    FROM tags
                    WHERE tags.name LIKE 'artist:%'
                ) AS artist_tags
            WHERE
                image_taggings.image_id = images.id
                AND image_taggings.tag_id = good_tags.id
                AND image_taggings.tag_id = bad_tags.id
                AND image_taggings.tag_id = artist_tags.id
            GROUP BY
                images.id
            HAVING
                COUNT(DISTINCT good_tags) = array_length($1::text[], 1)
                AND COUNT(bad_tags) = 0
            ORDER BY
                random()
            LIMIT
                50;
        "###,
            &[&terms, &blacklist],
        )?
        .into_iter()
        .map(|row| Image {
            id: row.get("id"),
            tags: row.try_get("tags").unwrap_or_default(),
            description: String::new(),
            name: row.try_get("name").unwrap_or_default(),
            first_seen_at: row.try_get("time").unwrap_or_default(),
            representations: RepresentationList {
                tall: format!("{}tall.png", row.get::<_, &str>("path")),
            },
        })
        .collect();

    Ok(ImageResponse {
        total: 0, // TODO
        images,
    })
}
