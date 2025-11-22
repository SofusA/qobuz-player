use std::{
    collections::{BTreeMap, HashMap},
    path::Path,
};

use qobuz_player_controls::Status;
use tera::{Function, Tera, Value, from_value, to_value};

pub(crate) fn templates(root_dir: &Path) -> Tera {
    let dir = format!("{}/**/*.html", root_dir.to_str().unwrap());
    let mut tera = Tera::new(&dir).unwrap();

    tera.register_filter("play_pause_api_string", play_pause_api_string);
    tera.register_filter("mseconds_to_mm_ss", mseconds_to_mm_ss);

    tera
}

fn play_pause_api_string(a: &Value, _: &HashMap<String, Value>) -> Result<Value, tera::Error> {
    let a: Status = serde_json::from_value(a.clone())?;

    let res = match a {
        Status::Paused | Status::Buffering => "/api/play",
        Status::Playing => "/api/pause",
    };

    Ok(Value::String(res.into()))
}

fn mseconds_to_mm_ss(a: &Value, _: &HashMap<String, Value>) -> Result<Value, tera::Error> {
    let a: i64 = serde_json::from_value(a.clone())?;

    let seconds: i64 = a / 1000;

    let minutes = seconds / 60;
    let seconds = seconds % 60;
    let res = format!("{minutes:02}:{seconds:02}");

    Ok(Value::String(res))
}

// fn list_callback() -> impl Function {
//     Box::new(
//         move |args: &HashMap<String, Value>| -> tera::Result<Value> {
//             let template = args
//                 .get("template")
//                 .ok_or(tera::Error::call_function("list_callback", "template"))?;

//             let template: String = from_value(template.clone())?;

//             let index = args
//                 .get("index")
//                 .ok_or(tera::Error::call_function("list_callback", "index"))?;
//             let index: i64 = from_value(index.clone())?;

//             Ok(template.replace("@index", &format!("{index}")))

//             // match args.get("template") {
//             //     Some(val) => match from_value::<String>(val.clone()) {
//             //         Ok(v) => Ok(to_value(urls.get(&v).unwrap()).unwrap()),
//             //         Err(_) => Err("oops".into()),
//             //     },
//             //     None => Err("oops".into()),
//             // }
//         },
//     )
// }

// handlebars_helper!(mseconds_to_mm_ss: |a: i64| {
//     let seconds: i64= a / 1000;

//     let minutes = seconds / 60;
//     let seconds = seconds % 60;
//     format!("{minutes:02}:{seconds:02}")
// });

// handlebars_helper!(multiply: |a: i64, b: i64| a * b);

// handlebars_helper!(ternary: |cond: bool, a: String, b: String| {
//     match cond {
//         true => a,
//         false => b,
//     }
// });

// handlebars_helper!(list_callback: |template: String, index: i32| {
//     template.replace("@index", &format!("{index}"))
// });
