#[macro_use]
extern crate slog;
extern crate slog_async;
extern crate slog_term;
extern crate reqwest;
extern crate cookie;
extern crate kuchiki;

use kuchiki::traits::*;
use slog::Drain;
use cookie::{CookieJar, Cookie};
use reqwest::header::Headers;

use std::env;
// use std::io::{self, Write};

struct Identity {
    smfcookie: String,
}

fn main() {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::CompactFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();

    let log = slog::Logger::root(drain, o!("epreuve" => "start"));

    let args: Vec<_> = env::args().collect();

    if 2 == args.len() {
        let id = Identity { smfcookie: env::args().nth(1).unwrap() };

        match ep134(id, &log) {
            Ok(reponse) => info!(log, "Réponse pour l'épreuve 134 : {}", reponse),
            Err(erreur) => info!(log, "Erreur lors de la résolution de l'épreuve 134, {}", erreur)
        }

    } else {
        crit!(log, "{} <cookie>", args[0]);
    }
}

fn process_set_cookie(log: &slog::Logger, jar: &mut CookieJar, set_cookie: &reqwest::header::SetCookie) {
    for c in set_cookie.iter() {
        match Cookie::parse(c.as_str()) {
            Ok(cookie) => {
                // debug!(log, "registering cookie"; "name" => cookie.name().to_owned(), "value" => cookie.value().to_owned());
                jar.add(Cookie::new(cookie.name().to_owned(), cookie.value().to_owned()))
            },
            Err(_) => debug!(log, "Failed to parse cookie")
        }
    }
}

fn headers_cookie(_log: &slog::Logger, jar: &CookieJar) -> Headers {
    let mut headers = Headers::new();
    let mut ck = reqwest::header::Cookie::new();
    for cookie in jar.iter() {
        // debug!(_log, "passing cookie"; "name" => cookie.name().to_owned(), "value" => cookie.value().to_owned());
        ck.append(cookie.name().to_owned(), cookie.value().to_owned());
    }
    headers.set(ck);
    headers
}

fn ep134(id: Identity, log: &slog::Logger) -> Result<String, String> {

    let cookie = id.smfcookie.clone();
    let url = "https://www.newbiecontest.org";
    let url_p1 = "https://www.newbiecontest.org/epreuves/prog/prog1.php";
    let url_p2 = "https://www.newbiecontest.org/epreuves/prog/verifpr1.php";

    let mut cookiejar = CookieJar::new();
    let authentication = log.new(o!("authenticate" => "newbiecontest.org"));

    // Add SMFCookie89
    cookiejar.add(Cookie::new("SMFCookie89", cookie.to_owned()));

    let client = reqwest::Client::new();
    match client.get(url).headers(headers_cookie(&authentication, &cookiejar)).send() {
        Ok(o) => {
            // debug!(authentication, "Status: {}", o.status());
            // debug!(authentication, "Headers:\n{}", o.headers());
            match o.headers().get::<reqwest::header::SetCookie>() {
                Some(ref set_cookie) => process_set_cookie(&authentication, &mut cookiejar, set_cookie),
                None => info!(authentication, "No cookie for access")
            }
        },
        Err(_) => return Err("Can't access newbiecontest.org".to_owned())
    };

    info!(authentication, "Logged in newbiecontest.org");

    let epreuve = log.new(o!("épreuve" => 134));

    info!(epreuve, "[+] accessing 1st page {}", url_p1);

    let mut res = match client.get(url_p1).headers(headers_cookie(&authentication, &cookiejar)).send() {
        Ok(o) => o,
        Err(_) => return Err("Error in first step".to_owned())
    };

    let mut number: i64 = -1;

    match res.status() {
        reqwest::StatusCode::Ok => {
            match res.text() {
                Ok(txt_epreuve) => {
                    // debug!(epreuve, "Text first page"; "text" => &txt_epreuve);
                    let v: Vec<&str> = txt_epreuve.rsplit(":").collect();
                    if v.len() >= 2 {
                        number = v[0].trim().parse().unwrap_or(-99);
                    }
                },
                Err(_) => {
                    warn!(epreuve, "No data received");
                    return Err("Error getting the first number".to_owned());
                }
            }
        },
        s => {
            warn!(epreuve, "Received response status: {:?}", s);
            return Err("Error getting the first number".to_owned());
        },
    };

    if number < 0 {
        crit!(epreuve, "Couldn't parse random number given");
        return Err("Failed to retrieve the number".to_owned());
    }

    info!(epreuve, "[+] random number {}", number);

    let answer = format!("{}?solution={}", url_p2, number);

    info!(epreuve, "[+] responding 2nd page {}", answer);

    let reponse_epreuve;

    match client.get(answer.as_str()).headers(headers_cookie(&authentication, &cookiejar)).send() {
        Ok(mut o) => {
            match o.status() {
                reqwest::StatusCode::Ok => {
                    match o.text() {
                        Ok(rep_epreuve) => {
                            let document = kuchiki::parse_html().one(rep_epreuve.as_str());

                            // debug!(epreuve, "Text second page"; "text" => &rep_epreuve);

                            // CSS version
                            // debug!(epreuve, "Document HTML \n{:?}", document);
                            for css_match in document.select("p").unwrap() {
                                let as_node = css_match.as_node();
                                // debug!(epreuve, "HTML tag {:?}", as_node);
                                let text_node = as_node.first_child().unwrap();

                                let text = text_node.as_text().unwrap().borrow();
                                let start_ans = text.rfind(": ").unwrap_or(0);
                                info!(epreuve, "[+] answer [css] {}", text.get(start_ans+2..).unwrap_or(&text));
                            }

                            // Raw text processing
                            let start_ans = rep_epreuve.rfind(": ").unwrap_or(0);
                            let end_ans = rep_epreuve.rfind("<").unwrap_or(rep_epreuve.len());
                            reponse_epreuve = rep_epreuve.get(start_ans+2..end_ans).unwrap_or(rep_epreuve.as_str()).to_owned();
                            info!(epreuve, "[+] answer [raw text] {}", reponse_epreuve);
                        },
                        Err(_) => {
                            warn!(epreuve, "No data received");
                            return Err("Error getting the final response".to_owned());
                        }
                    }
                },
                s => {
                    warn!(epreuve, "Received response status: {:?}", s);
                    return Err("Error when replying the number".to_owned());
                },
            }
        },
        Err(_) => return Err("Error in second step".to_owned())
    };

    Ok(reponse_epreuve)
}
