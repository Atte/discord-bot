use color_eyre::eyre::Result;
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
            None,
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
            None,
        )
        .await
}

async fn mongo_ensure_indexes(
    db: &Database,
    collection: &str,
    indexes: Vec<(Document, bool)>,
) -> Result<()> {
    #[allow(unused_must_use)]
    {
        // ignore error: the collection might already exist
        db.create_collection(collection, None).await;
    }

    let collection = db.collection::<Document>(collection);
    collection.drop_indexes(None).await?; // YOLO
    collection
        .create_indexes(
            indexes.into_iter().map(|(spec, unique)| {
                IndexModel::builder()
                    .keys(spec)
                    .options(IndexOptions::builder().unique(unique).build())
                    .build()
            }),
            None,
        )
        .await?;

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

    Ok(())
}
