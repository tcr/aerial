use fetch_string;
use get_scripts;
use html5ever::rcdom::{RcDom};
use html5ever::{parse, one_input};
use hyper::client::response::Response;
use hyper::Client;
use hyper::header::Connection;
use hyper;
use regex::{Regex, quote};
use serde_json::error::Error as DeError;
use serde_json::error::ErrorCode as DeErrorCode;
use serde_json::Value;
use serde_json;
use std::default::Default;
use std::io::Read;
use tendril::{ByteTendril, ReadExt};
use url::form_urlencoded;

#[derive(Debug)]
enum SigAction {
  Reverse,
  Swap(usize),
  Slice(usize),
  Splice(usize),
}

pub fn fetch(id: &str) -> hyper::error::Result<Response> {
    let client = Client::new();

    let id = id.trim();

    // Creating an outgoing request.
    let mut res = client.get(&format!("https://www.youtube.com/watch?v={}", id))
        .header(Connection::close())
        .send().unwrap();

    let mut input = ByteTendril::new();
    res.read_to_tendril(&mut input).unwrap();
    let input = input.try_reinterpret().unwrap();

    let dom: RcDom = parse(one_input(input), Default::default());

    // println!("hi");

    let scripts = get_scripts(dom.document);
    let script = scripts.iter().filter(|x| x.find("ytplayer.config = ").is_some()).nth(0).unwrap();

    let idx = script.find("ytplayer.config = ").unwrap();
    let jsonhead = &script[idx + "ytplayer.config = ".len()..];
    let out: Value = match serde_json::from_str(jsonhead) {
        Ok(value) => value,
        Err(DeError::SyntaxError(DeErrorCode::TrailingCharacters, start, end)) => {
            let jsonfull = &jsonhead[start-1..end-1];
            match serde_json::from_str(jsonfull) {
                Ok(value) => value,
                err => {
                    panic!("couldn't parse json: {:?}", err);
                }
            }
        }
        err => {
            panic!("couldn't parse json: {:?}", err);
        }
    };
    
    println!("{}", out.lookup("args.title").unwrap().as_string().unwrap());
    println!("URL: https://www.youtube.com/watch?v={}", id);

    // println!("um");

    // let fmt_list: Vec<_> = out.lookup("args.fmt_list")
    //     .map(|l| l.as_string().unwrap().split(",").collect())
    //     .unwrap_or(vec![]);
    // let video_verticals = out.lookup("args.video_verticals");


    // println!("er");

    let mut fmts = vec![];
    if let Some(fmtstr) = out.lookup("args.url_encoded_fmt_stream_map") {
        fmts.extend(fmtstr.as_string().unwrap().split(","));
    }
    if let Some(fmtstr) = out.lookup("args.adaptive_fmts") {
        fmts.extend(fmtstr.as_string().unwrap().split(","));
    }
    let mut fmts: Vec<_> = fmts.iter().map(|x| form_urlencoded::parse(x.as_bytes())).collect();

    let go = fmts.pop().unwrap();

    fn fmt_key(key: &str, of: &[(String, String)]) -> Option<String> {
        of
            .iter()
            .filter(|pair| pair.0 == key)
            .map(|pair| pair.1.clone())
            .nth(0)
    }

    let assets_url = format!("http:{}", out.lookup("assets.js").unwrap().as_string().unwrap());
    let body = fetch_string(&assets_url);

    let mut url = fmt_key("url", &go).unwrap();
    if let Some(sig) = fmt_key("s", &go) {
        let sig = apply_signature(&sig, extract_tokens(&body));
        url = url + &format!("&signature={}", sig);
    }

    if url.find("ratebypass").is_none() {
        url = url + "&ratebypass=yes"
    }

    client.get(&url)
        .header(Connection::close())
        .send()
}

fn apply_signature(input: &str, actions: Vec<SigAction>) -> String {
    let mut sig: Vec<char> = input.chars().collect();
    for token in actions {
        match token {
            SigAction::Reverse => {
                sig.reverse();
            }
            SigAction::Splice(n) => {
                sig = sig[n..].to_vec();
            }
            SigAction::Slice(n) => {
                sig = sig[n..].to_vec(); 
            }
            SigAction::Swap(n) => {
                let len = sig.len();
                sig.swap(0, n % len);
            }
        }
    }

    sig.into_iter().collect::<String>()
}

fn extract_tokens(body: &str) -> Vec<SigAction> {
    let jsvar_str = r"[a-zA-Z_\$][a-zA-Z_0-9]*";

    let reverse_str = concat!(r":function\(a\)\{",
      r"(?:return )?a\.reverse\(\)",
    r"\}");

    let slice_str = concat!(r":function\(a,b\)\{",
      r"return a\.slice\(b\)",
    r"\}");
    let splice_str = concat!(r":function\(a,b\)\{",
      r"a\.splice\(0,b\)",
    r"\}");
    let swap_str = concat!(r":function\(a,b\)\{",
      r"var c=a\[0\];a\[0\]=a\[b%a\.length\];a\[b\]=c(?:;return a)?",
    r"\}");

    let actions_obj_regexp = Regex::new(&(String::new() +
      r"var (" + jsvar_str + r")=\{((?:(?:" +
        jsvar_str + reverse_str + r"|" +
        jsvar_str + slice_str + r"|" +
        jsvar_str + splice_str + r"|" +
        jsvar_str + swap_str +
      r"),?\n?)+)\};"
    )).unwrap();
    let actions_func_regexp = Regex::new(&(String::new() +
      r"function(?: " + jsvar_str + r")?\(a\)\{" +
        r#"a=a\.split\(""\);\s*"# +
        r"((?:(?:a=)?" + jsvar_str + r"\." + jsvar_str + r"\(a,\d+\);)+)" +
        r#"return a\.join\(""\)"# +
      r"\}"
    )).unwrap();
    let reverse_regexp = Regex::new(&(String::new() + r"(?m)(?:^|,)(" + jsvar_str + r")" + reverse_str)).unwrap();
    let slice_regexp   = Regex::new(&(String::new() + r"(?m)(?:^|,)(" + jsvar_str + r")" + slice_str)).unwrap();
    let splice_regexp  = Regex::new(&(String::new() + r"(?m)(?:^|,)(" + jsvar_str + r")" + splice_str)).unwrap();
    let swap_regexp    = Regex::new(&(String::new() + r"(?m)(?:^|,)(" + jsvar_str + r")" + swap_str)).unwrap();

    // Regex::new(r"[a-zA-Z_\$][a-zA-Z_0-9]*");

    // var objResult = actionsObjRegexp.exec(body);
    // if (!objResult) { return null; }
    // var funcResult = actionsFuncRegexp.exec(body);
    // if (!funcResult) { return null; }


    let obj_result = actions_obj_regexp.captures(&body).unwrap();
    let func_result = actions_func_regexp.captures(&body).unwrap();

    let obj      = obj_result.at(1).unwrap();//.replace("$", r"\$");
    let obj_body  = obj_result.at(2).unwrap();//.replace("$", r"\$");
    let func_body = func_result.at(1).unwrap();//.replace("$", r"\$");

    // println!("{:?}\n{:?}\n{:?}", obj, objBody, funcbody);



    let reverse_key: &str = reverse_regexp.captures(&obj_body).map_or("", |x| x.at(1).unwrap());
    let slice_key: &str = slice_regexp.captures(&obj_body).map_or("", |x| x.at(1).unwrap());
    let splice_key: &str = splice_regexp.captures(&obj_body).map_or("", |x| x.at(1).unwrap());
    let swap_key: &str = swap_regexp.captures(&obj_body).map_or("", |x| x.at(1).unwrap());

    // println!("reverse {:?}", reverseKey);
    // println!("slice   {:?}", sliceKey);
    // println!("splice  {:?}", spliceKey);
    // println!("swap    {:?}", swapKey);

    let pipes: Vec<String> = vec![reverse_key, slice_key, splice_key, swap_key].iter().filter(|x| x.len() > 0).map(|x| quote(x)).collect();

    let tokenize_regexp = Regex::new(&(String::new() + r"(?:a=)?" + obj + r"\.(" + &pipes.join("|") + r")\(a,(\d+)\)")).unwrap();

    // println!("TOKENS {:?}", tokenizeRegexp);

    let mut tokens: Vec<SigAction> = vec![];
    for result in tokenize_regexp.captures_iter(&func_body) {
        let value = result.at(1).unwrap();
        if value == swap_key {
            tokens.push(SigAction::Swap(result.at(2).unwrap().parse().unwrap()));
        }
        if value == reverse_key {
            tokens.push(SigAction::Reverse);
        }
        if value == slice_key {
            tokens.push(SigAction::Slice(result.at(2).unwrap().parse().unwrap()));
        }
        if value == splice_key {
            tokens.push(SigAction::Splice(result.at(2).unwrap().parse().unwrap()));
        }
    }

    // println!("lol {:?}", tokens);

    tokens
}

#[test]
fn test_extract_tokens() {
    let body = fetch_string("http://s.ytimg.com/yts/jsbin/player-en_US-vflGR-A-c/base.js");
    let tokens = extract_tokens(&body);
    assert!(!tokens.is_empty());
}
