use std::collections::HashMap;
use std::{env, fmt, fs};

#[derive(Debug, Default)]
pub struct HyprlandConfig {
    pub content: Vec<String>,
    pub sections: HashMap<String, (usize, usize)>,
    pub sourced_content: Vec<Vec<String>>,
    pub sourced_sections: HashMap<String, (usize, usize)>,
    pub sourced_paths: Vec<String>,
}

impl HyprlandConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn parse(&mut self, config_str: &str, sourced: bool) {
        let mut section_stack = Vec::new();
        let mut sourced_content: Vec<String> = Vec::new();
        let source_index = if sourced {
            self.sourced_content.len()
        } else {
            0
        };

        let mut env_vars = HashMap::new();
        let home = env::var("HOME").unwrap_or_default();
        env_vars.insert("HOME".to_string(), home.clone());

        println!("Parsing env vars from config:");
        for line in config_str.lines() {
            let trimmed = line.trim();
            if let Some((var, val)) = trimmed
                .split_once('=')
                .map(|(v, p)| (v.trim(), p.split('#').next().unwrap_or(p).trim()))
            {
                if let Some(stripped) = var.strip_prefix('$') {
                    println!("Found env var: {} = {}", var, val);
                    let mut expanded_val = val.to_string();
                    for (existing_var, existing_val) in &env_vars {
                        expanded_val =
                            expanded_val.replace(&format!("${}", existing_var), existing_val);
                    }
                    env_vars.insert(stripped.to_string(), expanded_val);
                    continue;
                }
            }
        }
        println!("Collected env vars: {:?}", env_vars);

        for (i, line) in config_str.lines().enumerate() {
            let trimmed = line.trim();

            if trimmed.starts_with("source") && !sourced {
                if let Some(path) = trimmed
                    .split_once('=')
                    .map(|(_, p)| p.split('#').next().unwrap_or(p).trim())
                {
                    println!("Processing source path: {}", path);
                    let mut expanded_path = path.to_string();

                    for (var, val) in &env_vars {
                        let var_pattern = format!("${}", var);
                        println!("Replacing {} with {}", var_pattern, val);
                        expanded_path = expanded_path.replace(&var_pattern, val);
                    }
                    println!("After env var expansion: {}", expanded_path);

                    if !expanded_path.starts_with('/') && !expanded_path.starts_with('~') {
                        expanded_path = format!("{}/.config/hypr/{}", home, expanded_path);
                    } else {
                        expanded_path = expanded_path.replacen("~", &home, 1);
                    }
                    println!("Final expanded path: {}", expanded_path);

                    match fs::read_to_string(&expanded_path) {
                        Ok(content) => {
                            println!("Successfully read sourced file");
                            self.parse(&content, true);
                            self.sourced_paths.push(expanded_path);
                        }
                        Err(e) => println!("Failed to read file: {}", e),
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
                    self.sourced_sections
                        .insert(format!("{}_{}", full_name, source_index), (start, i));
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
        if sourced {
            self.sourced_content.push(sourced_content);
        }
    }

    fn detect_indentation(&self, content: &[String]) -> usize {
        for line in content {
            if let Some(indent) = line
                .chars()
                .take_while(|c| c.is_whitespace())
                .count()
                .checked_sub(0)
            {
                if indent > 0 {
                    return indent;
                }
            }
        }
        4
    }

    pub fn add_entry(&mut self, category: &str, entry: &str) {
        let parts: Vec<&str> = category.split('.').collect();
        let parent_category = if parts.len() > 1 {
            parts[..parts.len() - 1].join(".")
        } else {
            category.to_string()
        };

        if let Some((source_index, _)) = self.find_sourced_section(&parent_category) {
            let section_key = format!("{}_{}", parent_category, source_index);
            let (start, mut end) = *self.sourced_sections.get(&section_key).unwrap();
            let depth = parent_category.matches('.').count();
            let key = entry.split('=').next().unwrap().trim();

            let indent_size = if let Some(content) = self.sourced_content.get(source_index) {
                self.detect_indentation(content)
            } else {
                4
            };

            let mut should_update_sections = false;
            let mut content_updated = String::new();

            if let Some(sourced_content) = self.sourced_content.get_mut(source_index) {
                let subcategory_key = format!("{}_{}", category, source_index);

                if parts.len() > 1 && !self.sourced_sections.contains_key(&subcategory_key) {
                    let last_part = parts.last().unwrap();
                    let section_start =
                        format!("{}{} {{", " ".repeat(indent_size * (depth + 1)), last_part);
                    let section_end = format!("{}}}", " ".repeat(indent_size * (depth + 1)));

                    if end > 0 && !sourced_content[end - 1].trim().is_empty() {
                        sourced_content.insert(end, String::new());
                        end += 1;
                    }

                    sourced_content.insert(end, section_start);
                    sourced_content.insert(
                        end + 1,
                        format!("{}{}", " ".repeat(indent_size * (depth + 2)), entry),
                    );
                    sourced_content.insert(end + 2, section_end);

                    self.sourced_sections
                        .insert(subcategory_key, (end + 1, end + 1));
                    should_update_sections = true;
                } else if let Some(&(sub_start, sub_end)) =
                    self.sourced_sections.get(&subcategory_key)
                {
                    let parent_category = if parts.len() > 1 {
                        parts[..parts.len()].join(".")
                    } else {
                        category.to_string()
                    };
                    let depth = parent_category.matches('.').count();
                    let formatted_entry =
                        format!("{}{}", " ".repeat(indent_size * (depth + 1)), entry);

                    let existing_line = sourced_content[sub_start..=sub_end]
                        .iter()
                        .position(|line| line.trim().starts_with(key));

                    match existing_line {
                        Some(line_num) => {
                            sourced_content[sub_start + line_num] = formatted_entry;
                        }
                        None => {
                            sourced_content.insert(sub_end, formatted_entry);
                            should_update_sections = true;
                        }
                    }
                } else {
                    let formatted_entry =
                        format!("{}{}", " ".repeat(indent_size * (depth + 1)), entry);
                    let existing_line = sourced_content[start..=end]
                        .iter()
                        .position(|line| line.trim().starts_with(key));

                    match existing_line {
                        Some(line_num) => {
                            sourced_content[start + line_num] = formatted_entry;
                        }
                        None => {
                            sourced_content.insert(end, formatted_entry);
                            should_update_sections = true;
                        }
                    }
                }

                content_updated = sourced_content.join("\n");
            }

            if should_update_sections {
                self.update_sourced_sections(source_index, end, 1);
            }

            if let Some(sourced_path) = self.sourced_paths.get(source_index) {
                if !sourced_path.is_empty() {
                    if let Err(e) = fs::write(sourced_path, content_updated) {
                        eprintln!("Failed to write to sourced file {}: {}", sourced_path, e);
                    }
                }
            }
            return;
        }

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

    pub fn add_sourced(&mut self, config: Vec<String>) {
        self.sourced_content.push(config);
        self.sourced_paths.push(String::new());
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

    fn update_sourced_sections(&mut self, source_index: usize, pos: usize, offset: usize) {
        for ((_, (start, end)), sourced_path) in self
            .sourced_sections
            .iter_mut()
            .filter(|(_, (start, _))| *start >= pos)
            .zip(self.sourced_paths.iter().skip(source_index))
        {
            if !sourced_path.is_empty() {
                if *start >= pos {
                    *start += offset;
                    *end += offset;
                } else if *end >= pos {
                    *end += offset;
                }
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

    fn find_sourced_section(&self, category: &str) -> Option<(usize, (usize, usize))> {
        for (idx, _) in self.sourced_content.iter().enumerate() {
            let section_key = format!("{}_{}", category, idx);
            if let Some(&section) = self.sourced_sections.get(&section_key) {
                if self.sourced_paths.get(idx).map_or(false, |p| !p.is_empty()) {
                    return Some((idx, section));
                }
            }
        }
        None
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
