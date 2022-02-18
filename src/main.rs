// use serde_json;
use smallvec::SmallVec;
use smartstring::alias::String;
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fmt::Display;
use std::fs;
use std::ops::Index;
use std::str::FromStr;
use std::time::SystemTime;

#[derive(Debug, PartialEq)]
enum Json {
    Object(JsonObject),
    Array(JsonArray),
    Number(f64),
    String(String),
    True,
    False,
    Null,
    // fuck undefined
}

type JsonArray = Vec<Json>;

type JsonObject = HashMap<String, Json>;

// type JsonWithStartCursor = (Json, usize);
#[derive(Debug)]
struct JsonWithStartCursor {
    json: Json,
    cursor: usize,
}

// unsafe impl smallvec::Array for JsonWithStartCursor {
//     type Item = JsonWithStartCursor;
//     fn size() -> usize {
//         mem::size_of::<JsonWithStartCursor>()
//     }
// }

impl Index<usize> for Json {
    type Output = Json;

    fn index(&self, idx: usize) -> &Self::Output {
        match self {
            Json::Array(array) => &array[idx],
            Json::Object(_) => panic!("obj can not index by nubmer"),
            _ => panic!("number & string can not index"),
        }
    }
}

impl Index<&str> for Json {
    type Output = Json;
    fn index(&self, key: &str) -> &Self::Output {
        match self {
            Json::Object(hashmap) => &hashmap[key],
            Json::Array(_) => panic!("array can not index by str"),
            _ => panic!("number & string can not index"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum State {
    ObjectOpen,
    ArrayOpen,
    NumberOpen,
    StringOpen,
    TrueOpen,
    FalseOpen,
    NullOpen,
}

// type StateWithCursor = (State, usize);
#[derive(Clone)]
struct StateWithCursor {
    state: State,
    cursor: usize,
}

// unsafe impl smallvec::Array for StateWithCursor {
//     type Item = StateWithCursor;
//     fn size() -> usize {
//         mem::align_of::<StateWithCursor>()
//     }
// }

#[derive(Debug, PartialEq, Clone)]
enum ObjectParseState {
    ParseKey,
    ParseValue,
}

// unsafe impl smallvec::Array for ObjectParseState {
//     type Item = ObjectParseState;
//     fn size() -> usize {
//         mem::size_of::<ObjectParseState>()
//     }
// }

#[derive(Debug)]
enum SyntaxError {
    NoEnd,
    UnexpectedChar(usize),
    Unknown((&'static str, usize)),
}

impl Error for SyntaxError {}

impl Display for SyntaxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            SyntaxError::NoEnd => write!(f, "Uncaught SyntaxError: Unexpected end of JSON"),
            SyntaxError::UnexpectedChar(pos) => write!(
                f,
                "Uncaught SyntaxError: Unexpected number in JSON at position: {}",
                pos
            ),
            SyntaxError::Unknown((s, pos)) => write!(f, "Unknown err '{}' at: {}", s, pos),
        }
    }
}

fn is_number(c: char) -> bool {
    c == '1'
        || c == '2'
        || c == '3'
        || c == '4'
        || c == '5'
        || c == '6'
        || c == '7'
        || c == '8'
        || c == '9'
        || c == '.'
}

fn is_space(c: char) -> bool {
    c == ' ' || c == '\n' || c == '\t'
}

const RUE: [char; 3] = ['r', 'u', 'e'];
const ALSE: [char; 4] = ['a', 'l', 's', 'e'];
const NULL: [char; 3] = ['u', 'l', 'l'];

fn compare_letter(bigger: &[char], smaller: &[char], cursor: usize) -> Result<(), SyntaxError> {
    for idx in 0..smaller.len() - 1 {
        if bigger[idx + cursor] != smaller[idx] {
            return Err(SyntaxError::UnexpectedChar(cursor));
        }
    }

    Ok(())
}

/**
 * walks and sounds like a javascript JSON.parse
 */
fn parse(input: &str) -> Result<Json, SyntaxError> {
    let chars = input.chars().collect::<Vec<char>>();
    let input_len = chars.len();
    if input_len == 0 {
        return Err(SyntaxError::NoEnd);
    }

    let mut cursor: usize = 0;
    let mut state_stack = SmallVec::<[StateWithCursor; 128]>::new();
    let mut json_result_stack = SmallVec::<[JsonWithStartCursor; 128]>::new();
    let mut object_parse_states = SmallVec::<[ObjectParseState; 128]>::new();

    while cursor < input_len {
        let c = chars[cursor];
        // push & pop at back, so stack top is the LAST
        let first_state = state_stack.last().cloned();
        let last_parse_state = object_parse_states.last().cloned();
        // may or may not change curr state
        let mut try_update_new_state = |cursor: usize| -> Option<State> {
            let mut state: Option<State> = None;
            let ch = chars[cursor];
            if is_number(ch) {
                state = Some(State::NumberOpen);
            } else if ch == '"' {
                state = Some(State::StringOpen);
            } else if ch == '{' {
                state = Some(State::ObjectOpen);
                object_parse_states.push(ObjectParseState::ParseKey);
            } else if ch == '[' {
                state = Some(State::ArrayOpen);
            } else if ch == 'n' {
                state = Some(State::NullOpen);
            } else if ch == 't' {
                state = Some(State::TrueOpen);
            } else if ch == 'f' {
                state = Some(State::FalseOpen);
            }

            if let Some(s) = &state {
                state_stack.push(StateWithCursor {
                    state: s.clone(),
                    cursor: cursor,
                });
            }

            state
        };

        if is_space(c) {
            // skip
            cursor += 1
        } else if let Some(StateWithCursor {
            state,
            cursor: prev_cursor,
        }) = first_state
        {
            match state {
                State::StringOpen => {
                    if c == '"' {
                        // StringClose
                        let start = prev_cursor + 1;
                        json_result_stack.push(JsonWithStartCursor {
                            json: Json::String(String::from_str(&input[start..cursor]).unwrap()),
                            cursor: prev_cursor,
                        });
                        state_stack.pop();
                    }
                    cursor += 1
                }
                State::ArrayOpen => {
                    if c == ']' {
                        // ArrayClose
                        // println!("{:?}", json_result_stack);
                        let last_state_cursor = state_stack.pop().unwrap().cursor;
                        let mut array = Vec::<Json>::with_capacity(128);
                        while !json_result_stack.is_empty()
                            && json_result_stack.last().unwrap().cursor > last_state_cursor
                        {
                            let json_item = json_result_stack.pop().unwrap().json;
                            array.push(json_item);
                        }
                        array.reverse();
                        json_result_stack.push(JsonWithStartCursor {
                            json: Json::Array(array),
                            cursor: last_state_cursor,
                        });
                        cursor += 1;
                    } else if c == ',' {
                        cursor += 1;
                        try_update_new_state(cursor);
                    } else {
                        try_update_new_state(cursor);
                        cursor += 1;
                    }
                }
                State::NumberOpen => {
                    if is_number(c) && cursor < input_len - 1 {
                        cursor += 1;
                    } else {
                        let number_end_pos: usize;
                        if cursor == input_len - 1 {
                            number_end_pos = cursor + 1;
                            cursor += 1;
                        } else {
                            number_end_pos = cursor;
                        }
                        if let Ok(number) = input[prev_cursor..number_end_pos].parse::<f64>() {
                            json_result_stack.push(JsonWithStartCursor {
                                json: Json::Number(number),
                                cursor: prev_cursor,
                            });
                            state_stack.pop();
                        } else {
                            return Err(SyntaxError::UnexpectedChar(cursor));
                        }
                    }
                }
                State::ObjectOpen => {
                    // println!("len: {}", object_parse_states.len());
                    if is_space(c) {
                        // skip space
                        cursor += 1
                    } else if c == '}' {
                        // ObjectClose
                        let last_state_cursor = state_stack.pop().unwrap().cursor;
                        let mut object_hash = JsonObject::new();
                        while !object_parse_states.is_empty()
                            && json_result_stack.last().unwrap().cursor > last_state_cursor
                        {
                            // value
                            object_parse_states.pop();
                            let val = json_result_stack.pop().unwrap();

                            // key
                            object_parse_states.pop();
                            let key = json_result_stack.pop();

                            if let Some(JsonWithStartCursor {
                                json: Json::String(s),
                                cursor: _,
                            }) = key
                            {
                                object_hash.insert(s, val.json);
                            } else {
                                dbg!("{} {:?}", c, key);
                                return Err(SyntaxError::Unknown((
                                    "object key not string",
                                    cursor,
                                )));
                            }
                        }

                        json_result_stack.push(JsonWithStartCursor {
                            json: Json::Object(object_hash),
                            cursor,
                        });
                        cursor += 1;
                    } else if c == ':' {
                        // start parse value
                        match last_parse_state {
                            None => return Err(SyntaxError::Unknown(("no last state", cursor))),
                            Some(ObjectParseState::ParseValue) => {
                                return Err(SyntaxError::Unknown(("last state was value!", cursor)))
                            }
                            _ => {}
                        }

                        object_parse_states.push(ObjectParseState::ParseValue);
                        cursor += 1;
                    } else if c == ',' {
                        // start parse key
                        cursor += 1;
                        object_parse_states.push(ObjectParseState::ParseKey);
                    // } else if object_parse_states.is_empty() {
                    //     // start parse key
                    //     object_parse_states.push_back(ObjectParseState::ParseKey);
                    } else if last_parse_state == Some(ObjectParseState::ParseValue) {
                        // cont parse value, the real value may or maynot follow the ':',
                        // may have to skip some spaces
                        // eg: {"key":    value}
                        try_update_new_state(cursor);
                        cursor += 1;
                    } else if last_parse_state == Some(ObjectParseState::ParseKey) {
                        let next_state = try_update_new_state(cursor);
                        if next_state != None && next_state != Some(State::StringOpen) {
                            return Err(SyntaxError::UnexpectedChar(cursor));
                        }
                        cursor += 1;
                    } else {
                        println!("{}", c);
                        return Err(SyntaxError::Unknown(("unknow object char", cursor)));
                    }
                }
                State::TrueOpen => match compare_letter(&chars, &RUE, cursor) {
                    Ok(()) => {
                        cursor += RUE.len();
                        json_result_stack.push(JsonWithStartCursor {
                            json: Json::True,
                            cursor,
                        });
                        state_stack.pop();
                    }
                    Err(err) => {
                        return Err(err);
                    }
                },
                State::FalseOpen => match compare_letter(&chars, &ALSE, cursor) {
                    Ok(()) => {
                        cursor += ALSE.len();
                        json_result_stack.push(JsonWithStartCursor {
                            json: Json::False,
                            cursor,
                        });
                        state_stack.pop();
                    }
                    Err(err) => {
                        return Err(err);
                    }
                },
                State::NullOpen => match compare_letter(&chars, &NULL, cursor) {
                    Ok(()) => {
                        cursor += NULL.len();
                        json_result_stack.push(JsonWithStartCursor {
                            json: Json::Null,
                            cursor,
                        });
                        state_stack.pop();
                    }
                    Err(err) => {
                        return Err(err);
                    }
                },
            }
        } else {
            try_update_new_state(cursor);
            cursor += 1;
        }
    }

    // println!("{:?}", state_stack);
    // println!("{:?}", json_result_stack);

    assert_eq!(state_stack.len(), 0, "state still open");
    assert_eq!(json_result_stack.len(), 1, "more than one root left");
    Ok(json_result_stack.pop().unwrap().json)
}

#[test]
fn root_string() {
    let ret = parse("\"2222\"").unwrap();
    assert_eq!(ret, Json::String(String::from("2222")))
}

#[test]
fn root_number() {
    let ret = parse("1221").unwrap();
    assert_eq!(ret, Json::Number(1221.0))
}

#[test]
fn root_array() {
    let ret = parse("[1, 2,  \"3\"]").unwrap();
    assert_eq!(ret[0], Json::Number(1.0));
    assert_eq!(ret[1], Json::Number(2.0));
    assert_eq!(ret[2], Json::String(String::from("3")))
}

#[test]
fn some_thing_real() -> Result<(), Box<dyn Error>> {
    let result = parse("{ \"a\":1, \"b\":\" 2\", \"c\": [] }")?;
    assert_eq!(result["a"], Json::Number(1.0));
    assert_eq!(result["b"], Json::String(String::from(" 2")));
    assert_eq!(result["c"], Json::Array(Vec::<Json>::new()));
    Ok(())
}

#[test]
fn nested_obj() -> Result<(), Box<dyn Error>> {
    let result = parse("{ \"a\": { \"a\":3.414, \"b\":\" 2\", \"c\": [] } }")?;
    assert_eq!(result["a"]["a"], Json::Number(3.414));
    assert_eq!(result["a"]["b"], Json::String(String::from(" 2")));
    assert_eq!(result["a"]["c"], Json::Array(Vec::<Json>::new()));
    Ok(())
}

#[test]
fn true_false_null() -> Result<(), Box<dyn Error>> {
    let result = parse("{\"a\":true, \"b\":false, \"c\":null}")?;
    assert_eq!(result["a"], Json::True);
    assert_eq!(result["b"], Json::False);
    assert_eq!(result["c"], Json::Null);
    Ok(())
}

const ITER: i32 = 10000;

fn main() -> Result<(), Box<dyn Error>> {
    smartstring::validate();
    let mut pwd = env::current_dir()?;
    pwd.push("foo.json");
    let json_content = fs::read_to_string(pwd.as_path())?;
    let earlier = SystemTime::now();
    for _ in 0..ITER {
        let _json = parse(&json_content)?;
        // assert_eq!(
        //     _json["web-app"]["servlet"][0]["servlet-name"],
        //     Json::String("cofaxCDS".into())
        // );

        // let _v: serde_json::Value = serde_json::from_str(json_content.as_str())?;
        // println!("{:?}", v["data"][1]["key"]);
    }
    println!(
        "good job {:?}ms",
        SystemTime::now()
            .duration_since(earlier)
            .unwrap()
            .as_millis()
    );
    Ok(())
}
