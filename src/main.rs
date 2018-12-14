#![feature(decl_macro, proc_macro_hygiene)]

use dotenv::dotenv;
use rand::distributions::Alphanumeric;
use rand::thread_rng;
use rand::Rng;
use rocket::http::{Cookie, Cookies};
use rocket::response::Redirect;
use rocket::*;
use rocket_contrib::templates::Template;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;

#[derive(Deserialize)]
struct AccessTokenResponse {
    access_token: String,
}

#[derive(Serialize)]
struct IndexViewContext {
    signed_in: bool,
}

#[get("/")]
fn index(mut cookies: Cookies) -> Template {
    Template::render(
        "index",
        IndexViewContext {
            signed_in: cookies.get_private("token").is_some(),
        },
    )
}

#[get("/login")]
fn login(mut cookies: Cookies) -> Redirect {
    if cookies.get_private("state").is_some() {
        cookies.remove_private(Cookie::named("state"));
    }

    println!("Generating state...");

    let state: String = thread_rng().sample_iter(&Alphanumeric).take(15).collect();

    println!("Saving state...");

    cookies.add_private(Cookie::new("state", state.clone()));

    println!("State saved!");

    println!("Redirecting to GitHub...");

    Redirect::to(format!(
        "https://github.com/login/oauth/authorize?client_id={}&state={}",
        env::var("CLIENT_ID").unwrap(),
        state
    ))
}

#[get("/callback?<code>&<state>")]
fn callback(mut cookies: Cookies, code: String, state: String) -> String {
    if state == cookies.get_private("state").unwrap().value() {
        cookies.remove_private(Cookie::named("state"));

        println!("Code received: {}", code);

        println!("Generating token...");

        let mut params = HashMap::new();

        params.insert("client_id", env::var("CLIENT_ID").unwrap());
        params.insert("client_secret", env::var("CLIENT_SECRET").unwrap());
        params.insert("code", code);
        params.insert("state", state.clone());

        let resp: AccessTokenResponse = reqwest::Client::new()
            .post("https://github.com/login/oauth/access_token")
            .header("Accept", "application/json")
            .form(&params)
            .send()
            .unwrap()
            .json()
            .unwrap();

        println!("Token received: {}", resp.access_token);

        println!("Storing token...");

        cookies.add_private(Cookie::new("token", resp.access_token));

        println!("Token stored!");

        format!("Sweet! Your state ({}) matches the state that GitHub sent! You have been authenticated!", state)
    } else {
        cookies.remove_private(Cookie::named("state"));
        println!("Code received: {}", code);
        format!("Uh-oh... Your state ({}) does not match the state that GitHub sent! Maybe try logging in again..?", state)
    }
}

#[get("/logout")]
fn logout(mut cookies: Cookies) -> Redirect {
    if cookies.get_private("token").is_some() {
        cookies.remove_private(Cookie::named("token"));

        Redirect::to(uri!(index))
    } else {
        Redirect::to(uri!(login))
    }
}

fn main() {
    dotenv().ok();
    rocket::ignite()
        .attach(Template::fairing())
        .mount("/", routes![index, login, callback, logout])
        .launch();
}
