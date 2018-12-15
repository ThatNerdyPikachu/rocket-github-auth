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

#[derive(Deserialize, Serialize)]
struct UserInfoResponse {
    login: Option<String>,
    name: Option<String>,
    company: Option<String>,
    location: Option<String>,
    bio: Option<String>,
    public_repos: Option<i32>,
    followers: Option<i32>,
    following: Option<i32>,
}

#[derive(Clone, Deserialize)]
struct UserEmail {
    email: String,
    primary: bool,
}

#[derive(Serialize)]
struct IndexViewContext {
    signed_in: bool,
}

#[derive(Serialize)]
struct RedirectViewContext {
    url: String,
}

#[derive(Serialize)]
struct UserViewContext {
    login: String,
    name: String,
    company: String,
    location: String,
    email: String,
    bio: String,
    public_repos: String,
    followers: String,
    following: String,
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

#[get("/login?<redirect_to>")]
fn login(mut cookies: Cookies, redirect_to: Option<String>) -> Redirect {
    if cookies.get_private("state").is_some() {
        cookies.remove_private(Cookie::named("state"));
    }

    if cookies.get_private("redirect_to").is_some() {
        cookies.remove_private(Cookie::named("redirect_to"));
    }

    if redirect_to.is_some() {
        cookies.add_private(Cookie::new("redirect_to", redirect_to.unwrap()));
    }

    let state: String = thread_rng().sample_iter(&Alphanumeric).take(15).collect();

    cookies.add_private(Cookie::new("state", state.clone()));

    Redirect::to(format!(
        "https://github.com/login/oauth/authorize?client_id={}&scopes=user&state={}",
        env::var("CLIENT_ID").unwrap(),
        state
    ))
}

#[get("/callback?<code>&<state>")]
fn callback(mut cookies: Cookies, code: String, state: String) -> Redirect {
    if state == cookies.get_private("state").unwrap().value() {
        cookies.remove_private(Cookie::named("state"));

        let mut params = HashMap::new();

        params.insert("client_id", env::var("CLIENT_ID").unwrap());
        params.insert("client_secret", env::var("CLIENT_SECRET").unwrap());
        params.insert("code", code);
        params.insert("state", state);

        let resp: AccessTokenResponse = reqwest::Client::new()
            .post("https://github.com/login/oauth/access_token")
            .header("Accept", "application/json")
            .form(&params)
            .send()
            .unwrap()
            .json()
            .unwrap();

        cookies.add_private(Cookie::new("token", resp.access_token));

        if cookies.get_private("redirect_to").is_some() {
            let v = String::from(cookies.get_private("redirect_to").unwrap().value());

            cookies.remove_private(Cookie::named("redirect_to"));

            Redirect::to(v)
        } else {
            Redirect::to("/")
        }
    } else {
        Redirect::to("/login")
    }
}

#[get("/logout")]
fn logout(mut cookies: Cookies) -> Redirect {
    if cookies.get_private("token").is_some() {
        cookies.remove_private(Cookie::named("token"));

        Redirect::to("/")
    } else {
        Redirect::to("/login")
    }
}

fn option_to_string(opt: &Option<String>) -> String {
    if opt.is_some() {
        opt.clone().unwrap()
    } else {
        String::from("None found!")
    }
}

fn option_i32_to_string(opt: Option<i32>) -> String {
    if opt.is_some() {
        opt.unwrap().to_string()
    } else {
        String::from("None found!")
    }
}

fn get_personal_email(emails: &[UserEmail]) -> UserEmail {
    for e in emails.iter() {
        if e.primary
            || e.email.as_str().ends_with("gmail.com")
            || e.email.as_str().ends_with("icloud.com")
            || e.email.as_str().ends_with("hotmail.com")
            || e.email.as_str().ends_with("yahoo.com")
        {
            return e.clone();
        }
    }

    emails[0].clone()
}

#[get("/user")]
fn user(mut cookies: Cookies) -> Template {
    if cookies.get_private("token").is_none() {
        Template::render(
            "redirect",
            RedirectViewContext {
                url: String::from("/login?redirect_to=/user"),
            },
        )
    } else {
        let profile_resp: UserInfoResponse = reqwest::Client::new()
            .get("https://api.github.com/user")
            .header(
                "Authorization",
                format!("token {}", cookies.get_private("token").unwrap().value()),
            )
            .send()
            .unwrap()
            .json()
            .unwrap();

        let emails_resp: Vec<UserEmail> = reqwest::Client::new()
            .get("https://api.github.com/user/emails")
            .header(
                "Authorization",
                format!("token {}", cookies.get_private("token").unwrap().value()),
            )
            .send()
            .unwrap()
            .json()
            .unwrap();

        Template::render(
            "user",
            UserViewContext {
                login: option_to_string(&profile_resp.login),
                name: option_to_string(&profile_resp.name),
                company: option_to_string(&profile_resp.company),
                location: option_to_string(&profile_resp.location),
                email: get_personal_email(&emails_resp).email,
                bio: option_to_string(&profile_resp.bio),
                public_repos: option_i32_to_string(profile_resp.public_repos),
                followers: option_i32_to_string(profile_resp.followers),
                following: option_i32_to_string(profile_resp.following),
            },
        )
    }
}

fn main() {
    dotenv().ok();
    rocket::ignite()
        .attach(Template::fairing())
        .mount("/", routes![index, login, callback, logout, user])
        .launch();

    println!("nice");
}
