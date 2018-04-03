#![feature(plugin)]
#![plugin(rocket_codegen)]
#![cfg_attr(feature = "cargo-clippy", allow(needless_pass_by_value))]

extern crate dotenv;
extern crate rocket;

#[macro_use]
extern crate diesel;
extern crate r2d2;
extern crate r2d2_diesel;

use std::process;

use diesel::mysql::MysqlConnection;
use r2d2_diesel::ConnectionManager;

type Pool = r2d2::Pool<ConnectionManager<MysqlConnection>>;

/// Initializes a database pool.
fn init_pool(database_url: &str) -> Pool {
    let manager = ConnectionManager::<MysqlConnection>::new(database_url);
    r2d2::Pool::new(manager).expect("error creating database pool")
}

mod db;

// Add an user to database
#[get("/email/<email>")]
fn add_user(connection: DbConn, email: String) -> String {
    match db::create_user(&connection, &email) {
        Ok(user) => format!("Added user [email: {}, token: {}]", user.email, user.token),
        Err(_) => format!("Error: cannot add '{}', already created", email),
    }
}

// Delete an user from database
#[get("/email/delete/<email>/<token>")]
fn remove_user(connection: DbConn, email: String, token: String) -> String {
    //let connection = establish_connection();

    match db::delete_user(&connection, &email, &token) {
        Ok(n) if n == 0 => "Not removed".to_string(),
        Ok(_) => "Bye".to_string(),
        Err(ref s) if s == "Not found" => "Not found".to_string(),
        Err(ref s) if s == "Forbidden" => "Forbidden".to_string(),
        Err(s) => format!("{}", s),
    }
}

#[get("/emails")]
fn get_users(connection: DbConn) -> String {
    match db::get_all_users(&connection) {
        Err(s) => format!("error {}", s),
        Ok(users) => {
            let mut response = "".to_string();
            for user in users {
                response.push_str(&user.email);
                response.push_str("\n")
            }
            response
        }
    }
}

fn main() {
    dotenv::dotenv().ok();

    let database_url = match std::env::var("DATABASE_URL") {
        Ok(s) => s,
        Err(_) => {
            eprintln!("error: the DATABASE_URL variable is not set.");
            process::exit(1);
        }
    };

    rocket::ignite()
        .manage(init_pool(&database_url))
        .mount("/", routes![add_user, remove_user, get_users])
        .launch();
}

use rocket::http::Status;
use rocket::request::{self, FromRequest};
use rocket::{Outcome, Request, State};
///Connection Guard
use std::ops::Deref;

// Connection request guard type: a wrapper around an r2d2 pooled connection.
pub struct DbConn(pub r2d2::PooledConnection<ConnectionManager<MysqlConnection>>);

/// Attempts to retrieve a single connection from the managed database pool. If
/// no pool is currently managed, fails with an `InternalServerError` status. If
/// no connections are available, fails with a `ServiceUnavailable` status.
impl<'a, 'r> FromRequest<'a, 'r> for DbConn {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> request::Outcome<DbConn, ()> {
        let pool = request.guard::<State<Pool>>()?;
        match pool.get() {
            Ok(conn) => Outcome::Success(DbConn(conn)),
            Err(_) => Outcome::Failure((Status::ServiceUnavailable, ())),
        }
    }
}

// For the convenience of using an &DbConn as an &MysqlConnection.
impl Deref for DbConn {
    type Target = MysqlConnection;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
