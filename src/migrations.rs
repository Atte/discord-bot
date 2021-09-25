use crate::Result;
use log::info;
use mongodb::{
    bson::{doc, Document},
    error::Result as MongoResult,
    options::UpdateOptions,
    results::UpdateResult,
    Database,
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
                    field: { "$toString": format!("${}", field) }
                }
            }],
            UpdateOptions::default(),
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
            doc! { format!("{}.0", field): { "$type": "number" } },
            vec![doc! {
                "$set": {
                    field: { "$map": {
                        "input": format!("${}", field),
                        "in": { "$toString": "$$this" }
                    } }
                }
            }],
            UpdateOptions::default(),
        )
        .await
}

pub async fn mongo(db: &Database) -> Result<()> {
    info!("Running MongoDB migrations...");
    mongo_number_to_string(db, "stats", "id").await?;
    mongo_number_to_string(db, "sticky-roles", "user_id").await?;
    mongo_number_to_string(db, "sticky-roles", "guild_id").await?;
    mongo_number_array_to_string_array(db, "sticky-roles", "role_ids").await?;
    Ok(())
}
