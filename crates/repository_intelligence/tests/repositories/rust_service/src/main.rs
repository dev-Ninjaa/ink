mod db;
mod models;
mod routes;

use crate::db;
use crate::models;
use crate::routes::health;
use crate::routes::user;

fn main() {
    health::check();
    user::handle();
}
