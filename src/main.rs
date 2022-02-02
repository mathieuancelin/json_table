use std::io::{self, BufRead};
use serde_json::Value;
use serde_json::map::Map;
use cli_table::{Cell, RowStruct, Row, Style, Table, TableStruct, print_stdout};
use clap::{arg, App};
use std::fs;
use std::cmp::Ordering;

fn column_names(m: &Map<String, Value>, possible_columns: Option<&str>) -> Vec<String> {
    match possible_columns {
        None => m.keys().map(|name| name.clone()).collect(),
        Some(cols) => cols.split(",").map(|n| n.trim().to_string()).collect(),
    }
}

fn read_source(possible_source: Option<&str>) -> String {
    let mut source = String::new();
    match possible_source {
        None => {
            let stdin = io::stdin();
            for line in stdin.lock().lines() {
                let data = line.expect("could not read from stdin");
                source = source + "\n" + &data.to_string();
            }
        }
        Some(path) => {
            match fs::read_to_string(path) {
                Ok(content) => source = source + &content,
                Err(e) => panic!("error while reading source from {}: {}", path, e),
            }
        }
    }
    source
}

fn print_arr(arr: &Vec<Value>, display_header: bool, take: usize, skip: usize, possible_columns: Option<&str>, sorted_column: Option<&str>, sort_ordering: &str) -> std::io::Result<()> {
    let first_object: &Map<String, Value> = arr.get(0).unwrap().as_object().unwrap();
    let mut row_names: Vec<String> = column_names(first_object, possible_columns);
    let mut names: Vec<String> = vec!("index".to_string());
    names.append(&mut row_names);
    let header: RowStruct = names.iter().map(|cell| cell.cell().bold(true)).row();
    let mut raw_rows: Vec<&Value> = arr.iter().filter(|row| row.is_object()).skip(skip).take(take).collect();
    let sorted_rows: Vec<&Value> = match sorted_column {
        None => raw_rows,
        Some(column) => {
            raw_rows.sort_by(|a, b| {
                let mut ordering =  Ordering::Equal;
                let col_a = a.get(column).unwrap();
                let col_b = b.get(column).unwrap();
                if col_a.is_boolean() {
                    if col_a.as_bool().is_some() && col_b.as_bool().is_some() {
                        ordering = col_a.as_bool().unwrap().partial_cmp(&col_b.as_bool().unwrap()).unwrap();
                    }
                } else if col_a.is_f64() {
                    if col_a.as_f64().is_some() && col_b.as_f64().is_some() {
                        ordering = col_a.as_f64().unwrap().partial_cmp(&col_b.as_f64().unwrap()).unwrap();
                    }
                } else if col_a.is_i64() {
                    if col_a.as_i64().is_some() && col_b.as_i64().is_some() {
                        ordering = col_a.as_i64().unwrap().partial_cmp(&col_b.as_i64().unwrap()).unwrap();
                    }
                } else if col_a.is_u64() {
                    if col_a.as_u64().is_some() && col_b.as_u64().is_some() {
                        ordering = col_a.as_u64().unwrap().partial_cmp(&col_b.as_u64().unwrap()).unwrap();
                    }
                } else if col_a.is_string() {
                    if col_a.as_str().is_some() && col_b.as_str().is_some() {
                        ordering = col_a.as_str().unwrap().partial_cmp(col_b.as_str().unwrap()).unwrap();
                    }
                }
                if sort_ordering == "desc" {
                    ordering.reverse()
                } else {
                    ordering
                }
            });
            raw_rows
        }
    };
    let mut counter = 0;
    let rows: Vec<RowStruct> = sorted_rows.iter().map(|row| {
        counter = counter + 1;
        let obj = row.as_object().unwrap();
        let mut cnames = vec!(counter.to_string() + "_____idx");
        let mut ccnames = column_names(first_object, possible_columns);
        cnames.append(&mut ccnames);
        cnames.iter().map(|name| {
            if name.ends_with("_____idx") {
                return name.replace("_____idx", "").cell();
            }
            match obj.get(name) {
                None => "".cell(),
                Some(value) => {
                    if value.is_boolean() {
                        value.as_bool().unwrap().cell()
                    } else if value.is_null() {
                        "null".cell()
                    } else if value.is_f64() {
                        value.as_f64().unwrap().cell()
                    } else if value.is_i64() {
                        value.as_i64().unwrap().cell()
                    } else if value.is_u64() {
                        value.as_u64().unwrap().cell()
                    } else if value.is_string() && name == "name" {
                        value.as_str().unwrap().replace(" {}", "").cell()
                    } else if value.is_string() {
                        value.as_str().unwrap().cell()
                    } else {
                        "not displayable".cell()
                    }
                }
            }                            
        }).row() 
    }).collect();
    let mut table: TableStruct = rows.table();
    if display_header {
        table = table.title(header);
    }
    print_stdout(table)
}

fn main() {

    let matches = App::new("json_table")
        .version("0.1.0")
        .about("Display array of json objects as an ascii table")
        .arg(arg!(-c --columns <VALUE> "the columns to display separated by a colon. if none provided, all columns will be displayed").required(false))
        .arg(arg!(-t --take <VALUE> "the number of lines to display. if none provided, all lines will be displayed").required(false).validator(|s| s.parse::<i64>()))
        .arg(arg!(-s --skip <VALUE> "the number of lines to skip. if none provided, all lines will be displayed").required(false).validator(|s| s.parse::<i64>()))
        .arg(arg!(--sort <VALUE> "the column name to sort").required(false))
        .arg(arg!(--order <VALUE> "the sorting order. Default is asc").required(false))
        .arg(arg!(--page <VALUE> "the page to display").required(false).validator(|s| s.parse::<i64>()))
        .arg(arg!(--pagesize <VALUE> "the page size to display").required(false).validator(|s| s.parse::<i64>()))
        .arg(arg!([source] "optional source to operate on. if none provided, will read from stdin"))
        .get_matches();


    let mut take: usize = matches.value_of_t("take").unwrap_or(999999999);
    let mut skip: usize = matches.value_of_t("skip").unwrap_or(0);
    let page_size: usize = matches.value_of_t("pagesize").unwrap_or(10);
    match matches.value_of_t::<usize>("page") {
        Err(_) => (),
        Ok(page) => {
            let pagination_pos = (page - 1) * page_size;
            skip = pagination_pos;
            take = page_size;
        }
    };
    let possible_columns = matches.value_of("columns");
    let possible_source = matches.value_of("source");
    let sorted_column = matches.value_of("sort");
    let sort_ordering = matches.value_of("order").unwrap_or("asc");
    
    let source_content = read_source(possible_source);

    match serde_json::from_str::<Value>(&source_content) {
        Err(e) => panic!("error while parsing source into json. {}", e),
        Ok(source_document) => {
            if source_document.is_array() {
                let arr: &Vec<Value> = source_document.as_array().unwrap();
                if false { // TODO: add option to show page per page, more style
                    let max = arr.len();
                    let mut current = 10;
                    print_arr(arr, true, 10, 0, possible_columns, sorted_column, sort_ordering).unwrap();
                    while current < max {
                        let mut line = String::new();
                        let _ = std::io::stdin().read_line(&mut line).expect("Failed to read line");
                        print_arr(arr, false, 1, current, possible_columns, sorted_column, sort_ordering).unwrap();
                        current = current + 1;
                    }
                } else {
                  print_arr(arr, true, take, skip, possible_columns, sorted_column, sort_ordering).unwrap();
                }
            } else {
                panic!("the provided source is not a json array");
            }
        }
    }    
}
