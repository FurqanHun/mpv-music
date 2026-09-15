use super::items::MenuItem;
use skim::prelude::*;

pub fn run_skim_simple(items: Vec<&str>, prompt: &str) -> Option<String> {
    let skim_items: Vec<MenuItem> = items
        .into_iter()
        .map(|i| MenuItem {
            text: i.to_string(),
            id: "".to_string(),
        })
        .collect();

    let opts = SkimOptionsBuilder::default()
        .height("50%")
        .reverse(true)
        .prompt(prompt)
        //.typos(2)
        .inline_info(true)
        .build()
        .unwrap();

    let output = Skim::run_items(opts, skim_items).ok()?;
    if output.is_abort {
        return None;
    }

    output.selected_items.first().map(|i| i.text().to_string())
}

pub fn run_skim_input_prompt(prompt: &str, header: &str) -> Option<String> {
    let opts = SkimOptionsBuilder::default()
        .height("50%")
        .reverse(true)
        .prompt(prompt)
        .header(header)
        .inline_info(true)
        .build()
        .unwrap();

    let output = Skim::run_items(opts, Vec::<MenuItem>::new()).ok()?;
    if output.is_abort {
        return None;
    }

    let q = output.query.trim().to_string();
    if q.is_empty() { None } else { Some(q) }
}

pub fn run_skim_multi_selection(items: Vec<String>, prompt: &str) -> Option<Vec<String>> {
    let skim_items: Vec<MenuItem> = items
        .into_iter()
        .map(|i| MenuItem {
            text: i.clone(),
            id: i,
        })
        .collect();

    let opts = SkimOptionsBuilder::default()
        .height("50%")
        .reverse(true)
        .prompt(prompt)
        //.typos(2)
        .inline_info(true)
        .multi(true)
        .build()
        .unwrap();

    if let Ok(output) = Skim::run_items(opts, skim_items) {
        if output.is_abort {
            return None;
        }

        let selections: Vec<String> = output
            .selected_items
            .iter()
            .map(|i| i.text().to_string())
            .collect();

        if selections.is_empty() {
            None
        } else {
            Some(selections)
        }
    } else {
        None
    }
}
