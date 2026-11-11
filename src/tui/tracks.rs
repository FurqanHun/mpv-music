use super::icons::Icons;
use super::items::{DirItem, PlaylistItem, TrackItem};
use crate::config;
use crate::indexer;
use crate::player;
use anyhow::{Context, Result};
use skim::prelude::*;
use std::borrow::Borrow;
use std::collections::{HashMap, HashSet};

pub fn run_track_mode<T>(tracks: &[T], cfg: &config::Config, extra_args: &[String]) -> Result<()>
where
    T: Borrow<indexer::Track>,
{
    let icons = Icons::new(cfg.nerd_fonts);
    let skim_items: Vec<TrackItem> = tracks
        .iter()
        .filter_map(|item| {
            let track = item.borrow();
            if track.is_playlist() {
                return None;
            }
            let icon = if track.is_video() {
                icons.video()
            } else {
                icons.track()
            };

            Some(TrackItem::new(track.clone(), icon))
        })
        .collect();

    let track_prompt = icons.prompt(icons.track(), "Tracks");
    let opts = SkimOptionsBuilder::default()
        .height("100%")
        .multi(true)
        .preview("")
        .prompt(&track_prompt)
        .header("   Artist                Title")
        .reverse(true)
        .normalize(true)
        .typos(skim::Typos::Smart)
        .no_hscroll(true)
        .inline_info(true)
        .bind(super::multi_select_binds())
        .build()
        .unwrap();

    let output = super::runner::run_skim(opts, skim_items).context("Skim failed")?;
    if output.is_abort {
        return Ok(());
    }

    let paths: Vec<String> = output
        .selected_items
        .iter()
        .map(|i| i.output().to_string())
        .collect();
    if paths.is_empty() {
        return Ok(());
    }

    player::play_files(&paths, cfg, extra_args)?;
    Ok(())
}

pub fn build_dir_items(
    tracks: &[indexer::Track],
    music_dirs: &[std::path::PathBuf],
    icons: &Icons,
) -> Vec<DirItem> {
    let mut dir_map: HashMap<String, Vec<String>> = HashMap::new();

    for t in tracks {
        if let Some(parent) = std::path::Path::new(&t.path).parent() {
            let parent_str = parent.to_string_lossy().to_string();
            let file_name = std::path::Path::new(&t.path)
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "???".to_string());

            dir_map.entry(parent_str).or_default().push(file_name);
        }
    }

    let mut root_items = Vec::new();
    let mut handled_root_paths = HashSet::new();

    for root in music_dirs {
        let root_path = std::path::Path::new(root);
        let clean_root = root_path.components().as_path();
        let root_str = clean_root.to_string_lossy().to_string();
        let root_canon = root_path.canonicalize().ok();

        if handled_root_paths.contains(&root_str)
            || root_canon
                .as_ref()
                .is_some_and(|c| handled_root_paths.contains(&c.to_string_lossy().to_string()))
        {
            continue;
        }

        let mut sample_files = Vec::new();
        let mut total_tracks = 0;

        for t in tracks {
            let tp = std::path::Path::new(&t.path);
            let matches = tp.starts_with(clean_root)
                || root_canon.as_ref().is_some_and(|c| tp.starts_with(c));
            if matches {
                total_tracks += 1;
                if sample_files.len() < 20 {
                    let file_name = tp
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| "???".to_string());
                    sample_files.push(file_name);
                }
            }
        }

        if total_tracks > 0 {
            handled_root_paths.insert(root_str.clone());
            if let Some(ref canon) = root_canon {
                handled_root_paths.insert(canon.to_string_lossy().to_string());
            }

            let folder_name = clean_root
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| root_str.clone());

            root_items.push(DirItem::new(
                folder_name,
                root_str,
                total_tracks,
                sample_files,
                icons.folder(),
                true,
            ));
        }
    }

    root_items.sort_by(|a, b| crate::search::natural_cmp(&a.dirname, &b.dirname));

    let mut clean_roots: Vec<(std::path::PathBuf, Option<std::path::PathBuf>)> = music_dirs
        .iter()
        .map(|r| {
            let p = std::path::Path::new(r);
            let clean = p.components().as_path().to_path_buf();
            let canon = p.canonicalize().ok();
            (clean, canon)
        })
        .collect();
    // Prioritize deeper root paths if one root contains another
    clean_roots.sort_by_key(|a| std::cmp::Reverse(a.0.components().count()));

    let mut regular_items: Vec<DirItem> = dir_map
        .into_iter()
        .filter(|(path, _)| !handled_root_paths.contains(path))
        .map(|(path, files)| {
            let count = files.len();
            let path_obj = std::path::Path::new(&path);

            let rel_name = clean_roots.iter().find_map(|(clean, canon)| {
                if let Ok(rel) = path_obj.strip_prefix(clean) {
                    let s = rel.to_string_lossy().to_string();
                    if !s.is_empty() {
                        return Some(s);
                    }
                }
                if let Some(rel) = canon.as_ref().and_then(|cp| path_obj.strip_prefix(cp).ok()) {
                    let s = rel.to_string_lossy().to_string();
                    if !s.is_empty() {
                        return Some(s);
                    }
                }
                None
            });

            let name = rel_name.unwrap_or_else(|| {
                path_obj
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.clone())
            });

            DirItem::new(name, path, count, files, icons.folder(), false)
        })
        .collect();

    regular_items.sort_by(|a, b| {
        let cmp = crate::search::natural_cmp(&a.dirname, &b.dirname);
        if cmp == std::cmp::Ordering::Equal {
            crate::search::natural_cmp(&a.path, &b.path)
        } else {
            cmp
        }
    });

    let mut skim_items = root_items;
    skim_items.extend(regular_items);
    skim_items
}

pub fn run_dir_mode(
    tracks: &[indexer::Track],
    cfg: &config::Config,
    extra_args: &[String],
) -> Result<()> {
    let icons = Icons::new(cfg.nerd_fonts);
    let skim_items = build_dir_items(tracks, &cfg.music_dirs, &icons);

    let folder_prompt = icons.prompt(icons.folder(), "Folders");
    let opts = SkimOptionsBuilder::default()
        .multi(true)
        .prompt(&folder_prompt)
        .header("   Directory Name")
        .reverse(true)
        .normalize(true)
        .typos(skim::Typos::Smart)
        .no_hscroll(true)
        .inline_info(true)
        .preview("")
        .bind(super::multi_select_binds())
        .build()
        .unwrap();

    let output = super::runner::run_skim(opts, skim_items).context("Skim failed")?;
    if output.is_abort {
        return Ok(());
    }

    let mut files = Vec::new();
    for item in output.selected_items {
        let dir = item.output();
        let dir_path = std::path::Path::new(dir.as_ref());
        let dir_canon = dir_path.canonicalize().ok();
        let mut dir_tracks = Vec::new();
        for t in tracks {
            let tp = std::path::Path::new(&t.path);
            if tp.starts_with(dir_path) || dir_canon.as_ref().is_some_and(|c| tp.starts_with(c)) {
                dir_tracks.push(t.path.clone());
            }
        }
        dir_tracks.sort_by(|a, b| crate::search::natural_cmp(a, b));
        for path in dir_tracks {
            if !files.contains(&path) {
                files.push(path);
            }
        }
    }
    if files.is_empty() {
        return Ok(());
    }
    player::play_files(&files, cfg, extra_args)
}

pub fn run_playlist_mode(
    tracks: &[indexer::Track],
    cfg: &config::Config,
    extra_args: &[String],
) -> Result<()> {
    let icons = Icons::new(cfg.nerd_fonts);
    let skim_items: Vec<PlaylistItem> = tracks
        .iter()
        .filter_map(|t| {
            if t.is_playlist() {
                let (count, lines) = if let Ok(content) = std::fs::read_to_string(&t.path) {
                    let playlist_dir = std::path::Path::new(&t.path)
                        .parent()
                        .unwrap_or_else(|| std::path::Path::new("."));

                    let all_valid_lines: Vec<String> = content
                        .lines()
                        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
                        .filter_map(|line| {
                            let line_trim = line.trim();

                            if line_trim.starts_with("http://")
                                || line_trim.starts_with("https://")
                                || line_trim.starts_with("ftp://")
                            {
                                return Some(line_trim.to_string());
                            }

                            let path = std::path::PathBuf::from(line_trim);

                            if path.is_absolute() {
                                if path.exists() {
                                    Some(line_trim.to_string())
                                } else {
                                    log::debug!(
                                        "Skipping non-existent path in playlist: {}",
                                        line_trim
                                    );
                                    None
                                }
                            } else {
                                match dunce::canonicalize(playlist_dir.join(&path)) {
                                    Ok(canonical) => Some(canonical.to_string_lossy().to_string()),
                                    Err(_) => {
                                        log::debug!(
                                            "Could not resolve relative path in playlist: {}",
                                            line_trim
                                        );
                                        None
                                    }
                                }
                            }
                        })
                        .collect();

                    let total = all_valid_lines.len();
                    let sample = all_valid_lines.into_iter().take(15).collect();

                    (total, sample)
                } else {
                    (0, vec!["(Could not read file)".to_string()])
                };

                Some(PlaylistItem::new(
                    t.title.clone(),
                    t.path.clone(),
                    count,
                    lines,
                    icons.playlist(),
                ))
            } else {
                None
            }
        })
        .collect();

    let playlist_prompt = icons.prompt(icons.playlist(), "Playlists");
    let opts = SkimOptionsBuilder::default()
        .multi(true)
        .prompt(&playlist_prompt)
        .reverse(true)
        .normalize(true)
        .typos(skim::Typos::Smart)
        .no_hscroll(true)
        .inline_info(true)
        .preview("")
        .build()
        .unwrap();

    let output = super::runner::run_skim(opts, skim_items).context("Skim failed")?;
    if output.is_abort {
        return Ok(());
    }

    let paths: Vec<String> = output
        .selected_items
        .iter()
        .map(|item| item.output().to_string())
        .collect();

    if paths.len() == 1 {
        player::play(&paths[0], cfg, extra_args)?;
    } else if !paths.is_empty() {
        player::play_files(&paths, cfg, extra_args)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_dir_tracks_natural_sorting() {
        let mut tracks = vec![
            "/music/Album/10 - Track 10.flac".to_string(),
            "/music/Album/1 - Track 1.flac".to_string(),
            "/music/Album/2 - Track 2.flac".to_string(),
            "/music/Album/03 - Track 3.flac".to_string(),
        ];

        tracks.sort_by(|a, b| crate::search::natural_cmp(a, b));

        assert_eq!(
            tracks,
            vec![
                "/music/Album/1 - Track 1.flac".to_string(),
                "/music/Album/2 - Track 2.flac".to_string(),
                "/music/Album/03 - Track 3.flac".to_string(),
                "/music/Album/10 - Track 10.flac".to_string(),
            ]
        );
    }

    #[test]
    fn test_dir_prefix_component_isolation() {
        use std::path::Path;

        let tracks = vec![
            "/music/Rock/01 - Song A.flac",
            "/music/Rock/02 - Song B.flac",
            "/music/Rock and Roll/01 - Other.flac",
            "/music/Rock/Subdir/01 - Nested.flac",
        ];

        let selected_dir = "/music/Rock";
        let dir_path = Path::new(selected_dir);

        let matched: Vec<&str> = tracks
            .into_iter()
            .filter(|t| Path::new(t).starts_with(dir_path))
            .collect();

        assert_eq!(
            matched,
            vec![
                "/music/Rock/01 - Song A.flac",
                "/music/Rock/02 - Song B.flac",
                "/music/Rock/Subdir/01 - Nested.flac",
            ]
        );
        assert!(!matched.contains(&"/music/Rock and Roll/01 - Other.flac"));
    }

    #[test]
    fn test_dir_multi_select_parent_and_child_no_duplicates() {
        use std::path::Path;

        let tracks = [
            "/music/Rock/01 - Song A.flac",
            "/music/Rock/Subdir/01 - Nested.flac",
        ];

        let selected_dirs = vec!["/music/Rock", "/music/Rock/Subdir"];

        let mut files = Vec::new();
        for dir in selected_dirs {
            let dir_path = Path::new(dir);
            let mut dir_tracks: Vec<&str> = tracks
                .iter()
                .copied()
                .filter(|t| Path::new(t).starts_with(dir_path))
                .collect();
            dir_tracks.sort_by(|a, b| crate::search::natural_cmp(a, b));
            for path in dir_tracks {
                if !files.contains(&path) {
                    files.push(path);
                }
            }
        }

        assert_eq!(
            files,
            vec![
                "/music/Rock/01 - Song A.flac",
                "/music/Rock/Subdir/01 - Nested.flac",
            ]
        );
    }

    #[test]
    fn test_build_dir_items_pinned_roots_and_nested_subfolders() {
        use std::path::PathBuf;

        let tracks = vec![
            indexer::Track {
                path: "/music/musicdd/Ado/Album1/01 - Song.flac".to_string(),
                ..Default::default()
            },
            indexer::Track {
                path: "/music/musicdd/Ado/Album1/02 - Song.flac".to_string(),
                ..Default::default()
            },
            indexer::Track {
                path: "/music/musicdd/Ado/Album2/01 - Other.flac".to_string(),
                ..Default::default()
            },
            indexer::Track {
                path: "/music/OtherRoot/Single.flac".to_string(),
                ..Default::default()
            },
        ];

        let music_dirs = vec![
            PathBuf::from("/music/musicdd"),
            PathBuf::from("/music/OtherRoot"),
        ];

        let icons = Icons::new(crate::config::NerdFontMode::None);
        let items = build_dir_items(&tracks, &music_dirs, &icons);

        // Pinned roots must be first
        assert_eq!(items[0].dirname, "[Root] musicdd");
        assert_eq!(items[0].count, 3);
        assert!(items[0].is_root);
        assert_eq!(items[0].text(), "[Root] musicdd (3)");

        assert_eq!(items[1].dirname, "[Root] OtherRoot");
        assert_eq!(items[1].count, 1);
        assert!(items[1].is_root);
        assert_eq!(items[1].text(), "[Root] OtherRoot (1)");

        // Followed by regular subdirectories relative to root
        assert_eq!(items[2].dirname, "Ado/Album1");
        assert_eq!(items[2].count, 2);
        assert!(!items[2].is_root);
        assert_eq!(items[2].text(), "Ado/Album1 (2)");

        assert_eq!(items[3].dirname, "Ado/Album2");
        assert_eq!(items[3].count, 1);
        assert!(!items[3].is_root);
        assert_eq!(items[3].text(), "Ado/Album2 (1)");

        // OtherRoot directly contained Single.flac, so it should NOT be duplicated as a regular directory
        assert_eq!(items.len(), 4);
    }

    #[test]
    fn test_build_dir_items_relative_disambiguation() {
        use std::path::PathBuf;

        let tracks = vec![
            indexer::Track {
                path: "/music/Root/Queen/Greatest Hits/01 - Bohemian.flac".to_string(),
                ..Default::default()
            },
            indexer::Track {
                path: "/music/Root/Tupac/Greatest Hits/01 - Changes.flac".to_string(),
                ..Default::default()
            },
            indexer::Track {
                path: "/music/Root/Ado/Kyogen/01 - Readymade.flac".to_string(),
                ..Default::default()
            },
        ];

        let music_dirs = vec![PathBuf::from("/music/Root")];
        let icons = Icons::new(crate::config::NerdFontMode::None);
        let items = build_dir_items(&tracks, &music_dirs, &icons);

        assert_eq!(items[0].dirname, "[Root] Root");
        assert_eq!(items[1].dirname, "Ado/Kyogen");
        assert_eq!(items[2].dirname, "Queen/Greatest Hits");
        assert_eq!(items[3].dirname, "Tupac/Greatest Hits");
    }

    #[test]
    fn test_build_dir_items_nested_roots_prefer_deepest() {
        use std::path::PathBuf;

        let tracks = vec![indexer::Track {
            path: "/music/Base/Rock/Classic/01 - Song.flac".to_string(),
            ..Default::default()
        }];

        // Two roots where one is parent of the other
        let music_dirs = vec![
            PathBuf::from("/music/Base"),
            PathBuf::from("/music/Base/Rock"),
        ];

        let icons = Icons::new(crate::config::NerdFontMode::None);
        let items = build_dir_items(&tracks, &music_dirs, &icons);

        // Deeper root /music/Base/Rock should strip prefix for Classic, giving "Classic" instead of "Rock/Classic"
        let classic_item = items.iter().find(|i| !i.is_root).unwrap();
        assert_eq!(classic_item.dirname, "Classic");
    }

    #[test]
    fn test_build_dir_items_empty_roots_omitted() {
        use std::path::PathBuf;

        let tracks = vec![indexer::Track {
            path: "/music/Active/01 - Song.flac".to_string(),
            ..Default::default()
        }];

        let music_dirs = vec![
            PathBuf::from("/music/Active"),
            PathBuf::from("/music/NonExistentOrEmpty"),
        ];

        let icons = Icons::new(crate::config::NerdFontMode::None);
        let items = build_dir_items(&tracks, &music_dirs, &icons);

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].dirname, "[Root] Active");
    }

    #[test]
    fn test_build_dir_items_trailing_slashes_and_duplicates() {
        use std::path::PathBuf;

        let tracks = vec![
            indexer::Track {
                path: "/music/Active/01 - Song.flac".to_string(),
                ..Default::default()
            },
            indexer::Track {
                path: "/music/Active/Sub/02 - Nested.flac".to_string(),
                ..Default::default()
            },
        ];

        // Has trailing slash and duplicate
        let music_dirs = vec![
            PathBuf::from("/music/Active/"),
            PathBuf::from("/music/Active"),
        ];

        let icons = Icons::new(crate::config::NerdFontMode::None);
        let items = build_dir_items(&tracks, &music_dirs, &icons);

        // Exactly one root item and one subfolder
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].dirname, "[Root] Active");
        assert_eq!(items[0].count, 2);
        assert!(items[0].is_root);
        assert_eq!(items[1].dirname, "Sub");
        assert_eq!(items[1].count, 1);
    }
}
