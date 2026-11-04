use super::items::MenuItem;
use skim::matcher::Matcher;
use skim::prelude::*;
use std::rc::Rc;

pub fn align_tui_query(query: &str) -> String {
    let mut aligned = query
        .replace("||", " | ")
        .replace(" OR ", " | ")
        .replace(',', " | ")
        .replace("&&", " ")
        .replace(" AND ", " ")
        .replace(" & ", " ")
        .replace(';', " ");

    if aligned.ends_with(" OR") {
        aligned.truncate(aligned.len() - 3);
        aligned.push_str(" |");
    } else if aligned.ends_with(" AND") {
        aligned.truncate(aligned.len() - 4);
        aligned.push(' ');
    }

    aligned
}

pub struct AlignedEngineFactory {
    inner: Rc<dyn MatchEngineFactory>,
}

impl MatchEngineFactory for AlignedEngineFactory {
    fn create_engine_with_case(&self, query: &str, case: CaseMatching) -> Box<dyn MatchEngine> {
        let aligned = align_tui_query(query);
        self.inner.create_engine_with_case(&aligned, case)
    }
}

pub fn run_skim<I, T>(opts: SkimOptions, items: I) -> Option<SkimOutput>
where
    I: IntoIterator<Item = T>,
    T: SkimItem,
{
    let (factory, rank_builder) = Matcher::create_engine_factory_with_builder(&opts);
    let aligned_factory = Rc::new(AlignedEngineFactory { inner: factory });
    let matcher = Matcher::builder(aligned_factory)
        .rank_builder(rank_builder)
        .case(opts.case)
        .build();

    let mut skim = Skim::new_items(opts, items).ok()?;
    skim.app_mut().matcher = matcher;
    skim.app_mut().restart_matcher(true);

    if skim.should_enter() {
        let task = async {
            let _ = skim.enter().await;
            let _ = skim.run().await;
        };

        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            tokio::task::block_in_place(|| handle.block_on(task));
        } else if let Ok(rt) = tokio::runtime::Runtime::new() {
            rt.block_on(task);
        }
    }

    Some(skim.output())
}

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
        .normalize(true)
        .typos(skim::Typos::Smart)
        .no_hscroll(true)
        .inline_info(true)
        .build()
        .unwrap();

    let output = run_skim(opts, skim_items)?;
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
        .normalize(true)
        .inline_info(true)
        .build()
        .unwrap();

    let output = run_skim(opts, Vec::<MenuItem>::new())?;
    if output.is_abort {
        return None;
    }

    let q = output.query.trim().to_string();
    if q.is_empty() { None } else { Some(q) }
}

pub fn multi_select_binds() -> Vec<String> {
    vec![
        "ctrl-a:select-all".to_string(),
        "ctrl-d:deselect-all".to_string(),
    ]
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
        .normalize(true)
        .typos(skim::Typos::Smart)
        .no_hscroll(true)
        .inline_info(true)
        .multi(true)
        .bind(multi_select_binds())
        .build()
        .unwrap();

    if let Some(output) = run_skim(opts, skim_items) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_align_tui_query_operators() {
        assert_eq!(align_tui_query("Ado || Natori"), "Ado  |  Natori");
        assert_eq!(align_tui_query("Ado OR Natori"), "Ado | Natori");
        assert_eq!(align_tui_query("Ado, Natori"), "Ado |  Natori");
        assert_eq!(align_tui_query("Gentle && Ghosts"), "Gentle   Ghosts");
        assert_eq!(align_tui_query("Gentle AND Ghosts"), "Gentle Ghosts");
        assert_eq!(align_tui_query("Gentle & Ghosts"), "Gentle Ghosts");
        assert_eq!(align_tui_query("Gentle; Ghosts"), "Gentle  Ghosts");
        assert_eq!(align_tui_query("AT&T Band"), "AT&T Band");
    }

    struct DummyItem(&'static str);
    impl SkimItem for DummyItem {
        fn text(&self) -> Cow<'_, str> {
            Cow::Borrowed(self.0)
        }
    }

    fn match_candidates(query: &str, candidates: &[&'static str]) -> Vec<&'static str> {
        let opts = SkimOptionsBuilder::default()
            .normalize(true)
            .build()
            .unwrap();
        let (factory, _rank_builder) = Matcher::create_engine_factory_with_builder(&opts);
        let aligned_factory = Rc::new(AlignedEngineFactory { inner: factory });
        let engine = aligned_factory.create_engine_with_case(query, CaseMatching::Smart);

        candidates
            .iter()
            .copied()
            .filter(|&c| engine.match_item(&DummyItem(c)).is_some())
            .collect()
    }

    #[test]
    fn test_aligned_engine_factory_matching() {
        let candidates = &[
            "Ado - Usseewa",
            "Natori - Overdose",
            "The Gentle Men - Ghosts",
            "Eve - Dramaturgy",
            "AT&T Band - Live",
        ];

        assert_eq!(
            match_candidates("Ado, Natori", candidates),
            vec!["Ado - Usseewa", "Natori - Overdose"]
        );
        assert_eq!(
            match_candidates("Ado OR Gentle", candidates),
            vec!["Ado - Usseewa", "The Gentle Men - Ghosts"]
        );
        assert_eq!(
            match_candidates("Ado || Gentle", candidates),
            vec!["Ado - Usseewa", "The Gentle Men - Ghosts"]
        );
        assert_eq!(
            match_candidates("Gentle && Ghosts", candidates),
            vec!["The Gentle Men - Ghosts"]
        );
        assert_eq!(
            match_candidates("Gentle; Men", candidates),
            vec!["The Gentle Men - Ghosts"]
        );
        assert_eq!(
            match_candidates("Gentle AND Men", candidates),
            vec!["The Gentle Men - Ghosts"]
        );
        assert_eq!(
            match_candidates("Ado, Natori && Overdose", candidates),
            vec!["Natori - Overdose"]
        );
        assert_eq!(
            match_candidates("Ado OR", candidates),
            vec!["Ado - Usseewa"]
        );
        assert_eq!(
            match_candidates("Ado AND", candidates),
            vec!["Ado - Usseewa"]
        );
        assert_eq!(
            match_candidates("Band", candidates),
            vec!["AT&T Band - Live"]
        );
        assert_eq!(
            match_candidates("AT&T", candidates),
            vec!["AT&T Band - Live"]
        );
    }
}
