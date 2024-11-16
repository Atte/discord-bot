use color_eyre::eyre::Result;
use futures::TryStreamExt;
use log::info;
use mongodb::{
    bson::{doc, Document},
    error::Result as MongoResult,
    options::IndexOptions,
    results::UpdateResult,
    Database, IndexModel,
};

async fn mongo_number_to_string(
    db: &Database,
    collection: &str,
    field: &str,
) -> MongoResult<UpdateResult> {
    db.collection::<Document>(collection)
        .update_many(
            doc! { field: { "$type": "number" } },
            vec![doc! {
                "$set": {
                    field: { "$toString": format!("${field}") }
                }
            }],
        )
        .await
}

async fn mongo_number_array_to_string_array(
    db: &Database,
    collection: &str,
    field: &str,
) -> MongoResult<UpdateResult> {
    db.collection::<Document>(collection)
        .update_many(
            doc! { format!("{field}.0"): { "$type": "number" } },
            vec![doc! {
                "$set": {
                    field: { "$map": {
                        "input": format!("${field}"),
                        "in": { "$toString": "$$this" }
                    } }
                }
            }],
        )
        .await
}

async fn mongo_ensure_indexes(
    db: &Database,
    collection: &str,
    indexes: Vec<(Document, bool)>,
) -> Result<()> {
    // ignore error: the collection might already exist
    let _ = db.create_collection(collection).await;
    let collection = db.collection::<Document>(collection);

    let existing: Vec<_> = collection.list_indexes().await?.try_collect().await?;

    for (spec, unique) in &indexes {
        if existing.iter().any(|i| &i.keys == spec) {
            continue;
        }
        log::info!("Creating index {}", spec.to_string());
        collection
            .create_index(
                IndexModel::builder()
                    .keys(spec.clone())
                    .options(IndexOptions::builder().unique(*unique).build())
                    .build(),
            )
            .await?;
    }

    for index in existing {
        if indexes.iter().any(|(spec, _)| spec == &index.keys) {
            continue;
        }
        if let Some(name) = index.options.and_then(|options| options.name) {
            if name != "_id_" {
                log::warn!("Dropping index {} {}", name, index.keys.to_string());
                collection.drop_index(name).await?;
            }
        }
    }

    Ok(())
}

pub async fn mongo(db: &Database) -> Result<()> {
    info!("Running MongoDB migrations...");
    mongo_number_to_string(db, "stats", "id").await?;
    mongo_number_to_string(db, "sticky-roles", "user_id").await?;
    mongo_number_to_string(db, "sticky-roles", "guild_id").await?;
    mongo_number_array_to_string_array(db, "sticky-roles", "role_ids").await?;

    info!("Building MongoDB indexes...");
    mongo_ensure_indexes(
        db,
        "stats",
        vec![(doc! { "type": 1, "id": 1, "guild_id": 1 }, true)],
    )
    .await?;
    mongo_ensure_indexes(
        db,
        "sticky-roles",
        vec![(doc! { "guild_id": 1, "user_id": 1 }, true)],
    )
    .await?;
    mongo_ensure_indexes(
        db,
        "gib-seen",
        vec![(doc! { "image.id": 1 }, true), (doc! { "time": 1 }, false)],
    )
    .await?;
    mongo_ensure_indexes(
        db,
        "openai-user-log",
        vec![(doc! { "user_id": 1 }, false), (doc! { "time": 1 }, false)],
    )
    .await?;

    Ok(())
}
