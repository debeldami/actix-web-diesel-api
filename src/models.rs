use diesel::Queryable;
use serde::Serialize;

#[derive(Serialize, Debug, Queryable)]
pub struct Cat {
    pub id: i32,
    pub name: String,
    pub image_path: String,
}
