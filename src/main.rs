#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate lazy_static;

use canteen::{Canteen, Request, Response, Method};
use canteen::utils;

use postgres::{Client, NoTls};
use r2d2_postgres::PostgresConnectionManager;

type Date = chrono::NaiveDate;

lazy_static! {
    static ref DB_POOL: r2d2::Pool<PostgresConnectionManager<NoTls>> = {
        let manager = PostgresConnectionManager::new(
            "host=localhost user=jeff dbname=jeff".parse().unwrap(),
            NoTls,
        );

        r2d2::Pool::new(manager).unwrap()
    };
}

/* a full person record */
#[derive(Debug, Serialize, Deserialize)]
struct Person {
    #[serde(default)]
    id:         i32,
    first_name: String,
    last_name:  String,
    dob:        Date,
}

impl Person {
    pub fn from_row(row: postgres::row::Row) -> Person {
        Person {
            id:         row.get("id"),
            first_name: row.get("first_name"),
            last_name:  row.get("last_name"),
            dob:        row.get("dob"),
        }
    }
}

fn _person_response(conn: &mut Client, person_id: i32) -> Response {
    match conn.query("select id, first_name, last_name, dob from person where id = $1", &[&person_id]) {
        Ok(mut rows) => {
            match rows.pop() {
                Some(row) => {
                    let p = Person::from_row(row);
                    Response::as_json(&p)
                },
                None => utils::err_404_json("no results for given ID"),
            }
        },
        Err(e) => {
            utils::err_500_json(&format!("{:?}", e))
        }
    }
}

fn create_person(req: &Request) -> Response {
    let person_id: i32;
    let pers: Person = req.get_json_obj().unwrap();

    let mut conn = DB_POOL.clone().get().unwrap();
    let cur = conn.query("insert into person (first_name, last_name, dob)\
                          values ($1, $2, $3) returning id",
                          &[&pers.first_name, &pers.last_name, &pers.dob]);

    match cur {
        Ok(mut rows) => {
            match rows.pop() {
                Some(row) => { person_id = row.get("id"); },
                None => { return utils::err_500_json("person couldn\'t be created"); },
            }
        },
        Err(e) => {
            return utils::err_500_json(&format!("{:?}", e))
        }
    }

    _person_response(&mut conn, person_id)
}

fn get_many_person(_: &Request) -> Response {
    let mut conn = DB_POOL.clone().get().unwrap();
    let cur = conn.query("select id, first_name, last_name, dob from person order by id", &[]);

    match cur {
        Ok(rows) => {
            let people: Vec<Person> = rows.into_iter().map(Person::from_row).collect();
            Response::as_json(&people)

        },
        Err(e) => {
            utils::err_500_json(&format!("{:?}", e))
        }
    }
}

fn get_single_person(req: &Request) -> Response {
    let person_id: i32 = req.get("person_id");
    let mut conn = DB_POOL.clone().get().unwrap();

    _person_response(&mut conn, person_id)
}

fn hello_world(_: &Request) -> Response {
    utils::make_response("hello, world!", "text/plain", 200)
}

fn main() {
    let mut cnt = Canteen::new();
    cnt.bind(("127.0.0.1", 8080));

    cnt.set_default(utils::err_404);
    cnt.add_route("/", &[Method::Get], hello_world)
       .add_route("/person", &[Method::Post], create_person)
       .add_route("/person", &[Method::Get], get_many_person)
       .add_route("/person/<int:person_id>", &[Method::Get], get_single_person)
       .add_route("/src/<path:path>", &[Method::Get], utils::static_file);

    cnt.run();
}

