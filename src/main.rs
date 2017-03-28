extern crate canteen;
extern crate rustc_serialize;
extern crate postgres;
extern crate chrono;

use canteen::{Canteen, Request, Response, Method};
use canteen::utils;

use rustc_serialize::{Encoder, Encodable};
use rustc_serialize::{Decoder, Decodable};
use postgres::{Connection, TlsMode};

type Date = chrono::NaiveDate;

/* a full person record */
#[derive(Debug)]
struct Person {
    id:         i32,
    first_name: String,
    last_name:  String,
    dob:        Date,
}

impl Encodable for Person {
    fn encode<S: Encoder>(&self, s: &mut S) -> Result<(), S::Error> {
        s.emit_struct("Person", 4, |s| {
            s.emit_struct_field("id", 0, |s| { s.emit_i32(self.id) })?;
            s.emit_struct_field("first_name", 1, |s| { s.emit_str(&self.first_name) })?;
            s.emit_struct_field("last_name", 2, |s| { s.emit_str(&self.last_name) })?;
            s.emit_struct_field("dob", 3, |s| { s.emit_str(&self.dob.format("%Y-%m-%d").to_string()) })?;

            Ok(())
        })
    }
}

impl Decodable for Person {
    fn decode<D: Decoder>(d: &mut D) -> Result<Person, D::Error> {
        d.read_struct("root", 3, |d| {
            let first_name = d.read_struct_field("first_name", 0, |d| { d.read_str() })?;
            let last_name = d.read_struct_field("last_name", 0, |d| { d.read_str() })?;
            let pre_dob = d.read_struct_field("dob", 0, |d| { d.read_str() })?;

            match Date::parse_from_str(&pre_dob, "%Y-%m-%d") {
                Ok(dob) => {
                    Ok(Person {
                        id:         0,
                        first_name: first_name,
                        last_name:  last_name,
                        dob:        dob,
                    })
                },
                Err(_)  => {
                    Err(d.error("failed to parse date provided"))
                },
            }
        })

    }
}

fn _person_response(conn: &Connection, person_id: i32) -> Response {
    match conn.query("select id, first_name, last_name, dob from person where id = $1", &[&person_id]) {
        Ok(rows)    => {
            match rows.len() {
                1 => {
                    let row = rows.get(0);
                    let p = Person {
                        id:         row.get("id"),
                        first_name: row.get("first_name"),
                        last_name:  row.get("last_name"),
                        dob:        row.get("dob"),
                    };

                    Response::as_json(&p)
                },
                0 => utils::err_404_json("no results for given ID"),
                _ => utils::err_404_json("too many results for given ID"),
            }
        },
        Err(e)      => {
            utils::err_500_json(&format!("{:?}", e))
        }
    }
}

fn create_person(req: &Request) -> Response {
    let person_id: i32;
    let pers: Person = req.get_json_obj().unwrap();

    let conn = Connection::connect("postgresql://jeff@localhost/jeff", TlsMode::None).unwrap();
    let cur = conn.query("insert into person (first_name, last_name, dob)\
                          values ($1, $2, $3) returning id",
                          &[&pers.first_name, &pers.last_name, &pers.dob]);

    match cur {
        Ok(rows)    => {
            match rows.len() {
                1 => {
                    person_id = rows.get(0).get("id");
                },
                _ => {
                    return utils::err_500_json("person couldn\'t be created");
                },
            }
        },
        Err(e)      => {
            return utils::err_500_json(&format!("{:?}", e))
        }
    }

    _person_response(&conn, person_id)
}

fn get_many_person(_: &Request) -> Response {
    let conn = Connection::connect("postgresql://jeff@localhost/jeff", TlsMode::None).unwrap();
    let cur = conn.query("select id, first_name, last_name, dob from person order by id", &[]);

    match cur {
        Ok(rows)    => {
            let mut people: Vec<Person> = Vec::new();

            for row in rows.iter() {
                people.push(Person {
                    id:         row.get("id"),
                    first_name: row.get("first_name"),
                    last_name:  row.get("last_name"),
                    dob:        row.get("dob"),
                });
            }

            Response::as_json(&people)

        },
        Err(e)      => {
            utils::err_500_json(&format!("{:?}", e))
        }
    }
}

fn get_single_person(req: &Request) -> Response {
    let person_id: i32 = req.get("person_id");
    let conn = Connection::connect("postgresql://jeff@localhost/jeff", TlsMode::None).unwrap();

    _person_response(&conn, person_id)
}

fn hello_world(_: &Request) -> Response {
    utils::make_response("hello, world!", "text/plain", 200)
}

fn main() {
    let mut cnt = Canteen::new(("127.0.0.1", 8080));

    cnt.set_default(utils::err_404);
    cnt.add_route("/", &[Method::Get], hello_world)
       .add_route("/person", &[Method::Post], create_person)
       .add_route("/person", &[Method::Get], get_many_person)
       .add_route("/person/<int:person_id>", &[Method::Get], get_single_person)
       .add_route("/src/<path:path>", &[Method::Get], utils::static_file);

    cnt.run();
}

