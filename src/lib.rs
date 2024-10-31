use std::collections::HashMap;
use std::{env, fmt, fs};

#[derive(Debug, Default)]
pub struct HyprlandConfig {
    content: Vec<String>,
    sections: HashMap<String, (usize, usize)>,
    sourced_content: Vec<Vec<String>>,
    sourced_sections: HashMap<String, (usize, usize)>,
    sourced_paths: Vec<String>,
}

impl HyprlandConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn parse(&mut self, config_str: &str, sourced: bool) {
        let mut section_stack = Vec::new();
        let mut sourced_content: Vec<String> = Vec::new();

        for (i, line) in config_str.lines().enumerate() {
            let trimmed = line.trim();

            if trimmed.starts_with("source =") && !sourced {
                if let Some(sourced_path) = trimmed.strip_prefix("source =").map(|s| s.trim()) {
                    if !sourced_path.starts_with("/") && !sourced_path.starts_with("~") {
                        let sourced_path = format!(
                            "{}/.config/hypr/{}",
                            env::var("HOME").unwrap(),
                            sourced_path
                        );
                        self.parse(&fs::read_to_string(sourced_path.clone()).unwrap(), true);
                        self.sourced_paths.push(sourced_path);
                    } else {
                        let sourced_path =
                            sourced_path.replacen("~", &env::var("HOME").unwrap(), 1);
                        self.parse(&fs::read_to_string(sourced_path.clone()).unwrap(), true);
                        self.sourced_paths.push(sourced_path);
                    }
                }
            } else if trimmed.ends_with('{') {
                let section_name = trimmed.trim_end_matches('{').trim().to_string();
                section_stack.push((section_name, i));
            } else if trimmed == "}" && !section_stack.is_empty() {
                let (name, start) = section_stack.pop().unwrap();
                let full_name = section_stack
                    .iter()
                    .map(|(n, _)| n.as_str())
                    .chain(std::iter::once(name.as_str()))
                    .collect::<Vec<_>>()
                    .join(".");
                if sourced {
                    self.sourced_sections.insert(full_name, (start, i));
                } else {
                    self.sections.insert(full_name, (start, i));
                }
            }
            if sourced {
                sourced_content.push(line.to_string());
            } else {
                self.content.push(line.to_string());
            }
        }
        self.sourced_content.push(sourced_content);
    }

    pub fn add_entry(&mut self, category: &str, entry: &str) {
        let parts: Vec<&str> = category.split('.').collect();
        let mut current_section = String::new();
        let mut insert_pos = self.content.len();

        for (depth, (i, part)) in parts.iter().enumerate().enumerate() {
            if i > 0 {
                current_section.push('.');
            }
            current_section.push_str(part);

            if !self.sections.contains_key(&current_section) {
                self.create_category(&current_section, depth, &mut insert_pos);
            }

            let &(start, end) = self.sections.get(&current_section).unwrap();
            insert_pos = end;

            if i == parts.len() - 1 {
                let key = entry.split('=').next().unwrap().trim();
                let existing_line = self.content[start..=end]
                    .iter()
                    .position(|line| line.trim().starts_with(key))
                    .map(|pos| start + pos);

                let formatted_entry = format!("{}{}", "    ".repeat(depth + 1), entry);

                match existing_line {
                    Some(line_num) => {
                        self.content[line_num] = formatted_entry;
                    }
                    None => {
                        self.content.insert(end, formatted_entry);
                        self.update_sections(end, 1);
                    }
                }
                return;
            }
        }
    }

    pub fn add_entry_headless(&mut self, key: &str, value: &str) {
        if key.is_empty() && value.is_empty() {
            self.content.push(String::new());
        } else {
            let entry = format!("{} = {}", key, value);
            if !self.content.iter().any(|line| line.trim() == entry.trim()) {
                self.content.push(entry);
            }
        }
    }

    fn update_sections(&mut self, pos: usize, offset: usize) {
        for (start, end) in self.sections.values_mut() {
            if *start >= pos {
                *start += offset;
                *end += offset;
            } else if *end >= pos {
                *end += offset;
            }
        }
    }

    pub fn parse_color(&self, color_str: &str) -> Option<(f32, f32, f32, f32)> {
        if color_str.starts_with("rgba(") {
            let rgba = color_str.trim_start_matches("rgba(").trim_end_matches(')');
            let rgba = u32::from_str_radix(rgba, 16).ok()?;
            Some((
                ((rgba >> 24) & 0xFF) as f32 / 255.0,
                ((rgba >> 16) & 0xFF) as f32 / 255.0,
                ((rgba >> 8) & 0xFF) as f32 / 255.0,
                (rgba & 0xFF) as f32 / 255.0,
            ))
        } else if color_str.starts_with("rgb(") {
            let rgb = color_str.trim_start_matches("rgb(").trim_end_matches(')');
            let rgb = u32::from_str_radix(rgb, 16).ok()?;
            Some((
                ((rgb >> 16) & 0xFF) as f32 / 255.0,
                ((rgb >> 8) & 0xFF) as f32 / 255.0,
                (rgb & 0xFF) as f32 / 255.0,
                1.0,
            ))
        } else if let Some(stripped) = color_str.strip_prefix("0x") {
            let argb = u32::from_str_radix(stripped, 16).ok()?;
            Some((
                ((argb >> 16) & 0xFF) as f32 / 255.0,
                ((argb >> 8) & 0xFF) as f32 / 255.0,
                (argb & 0xFF) as f32 / 255.0,
                ((argb >> 24) & 0xFF) as f32 / 255.0,
            ))
        } else {
            None
        }
    }

    pub fn format_color(&self, red: f32, green: f32, blue: f32, alpha: f32) -> String {
        format!(
            "rgba({:02x}{:02x}{:02x}{:02x})",
            (red * 255.0) as u8,
            (green * 255.0) as u8,
            (blue * 255.0) as u8,
            (alpha * 255.0) as u8
        )
    }

    fn create_category(&mut self, category: &str, depth: usize, insert_pos: &mut usize) {
        let part = category.split('.').last().unwrap();
        let new_section = format!("{}{} {{", "    ".repeat(depth), part);

        let mut lines_added = 0;
        if *insert_pos > 0 && !self.content[*insert_pos - 1].trim().is_empty() {
            self.content.insert(*insert_pos, String::new());
            *insert_pos += 1;
            lines_added += 1;
        }

        self.content.insert(*insert_pos, new_section);
        *insert_pos += 1;
        self.content
            .insert(*insert_pos, format!("{}}}", "    ".repeat(depth)));
        *insert_pos += 1;
        self.content.insert(*insert_pos, String::new());
        *insert_pos += 1;

        self.update_sections(*insert_pos - 3 - lines_added, 3 + lines_added);
        self.sections.insert(
            category.to_string(),
            (*insert_pos - 3 - lines_added, *insert_pos - 2),
        );
    }
}

pub fn parse_config(config_str: &str) -> HyprlandConfig {
    let mut config = HyprlandConfig::new();
    config.parse(config_str, false);
    config
}

impl fmt::Display for HyprlandConfig {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (i, line) in self.content.iter().enumerate() {
            if i == self.content.len() - 1 {
                write!(f, "{}", line)?;
            } else {
                writeln!(f, "{}", line)?;
            }
        }
        Ok(())
    }
}

impl PartialEq for HyprlandConfig {
    fn eq(&self, other: &Self) -> bool {
        self.content == other.content
    }
}
