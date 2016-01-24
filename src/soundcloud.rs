use fetch_json;
use snitch;
use hyper::client::response::Response;
use hyper::Client;
use hyper::header::{Connection, UserAgent};
use hyper;
use regex::{Regex, is_match};

pub fn fetch(id: &str) -> hyper::error::Result<Response> {
    let client = Client::new();

    let mut sn_id: String = id.into();
    if !is_match(r"^\d+$", &sn_id).unwrap() {
	    // Creating an outgoing request.
	    let mut res = client.get(&format!("https://soundcloud.com/{}", id))
	        .header(Connection::close())
	        .header(UserAgent("hyper/0.5.2".into()))
	        .send().unwrap();

	    // println!("lol");
	    for url in snitch(&mut res, &Regex::new(r#"api\.soundcloud\.com/tracks/(\d+)\D"#).unwrap()) {
	    	sn_id = url.snitch().at(1).unwrap().into();
	    	break;
	    }
    }

    // println!("URL: https://api.soundcloud.com/tracks/{}", sn_id);

    let client_id = "c8ce5cbca9160b790311f06638a61037";

    let info = fetch_json(&format!("https://api.soundcloud.com/tracks/{}?client_id={}&format=json", sn_id, client_id)).unwrap();
    // println!("Track: {}", info.lookup("title").unwrap().as_string().unwrap());
    println!("URL: {}", info.lookup("permalink_url").unwrap().as_string().unwrap());

    let streams = fetch_json(&format!("https://api.soundcloud.com/tracks/{}/streams?client_id={}&format=json", sn_id, client_id)).unwrap();
    let mp3url = streams.lookup("http_mp3_128_url").unwrap().as_string().unwrap();

    client.get(mp3url)
        .header(Connection::close())
        .header(UserAgent("hyper/0.5.2".into()))
        .send()
}

#[test]
fn test_sn_fetch() {
	fetch("bluehawaii/agor-edits");
}
