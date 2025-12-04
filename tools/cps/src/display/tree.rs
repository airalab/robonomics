use colored::*;

/// Print a beautiful tree structure for a CPS node
pub fn print_tree(node_id: u64, owner: &str, meta: Option<&str>, payload: Option<&str>, children: &[u64]) {
    println!("\n{} {}\n", "🌳 CPS Node".bright_cyan().bold(), format!("ID: {}", node_id).bright_white().bold());
    
    // Owner
    println!("{}  {} {}", 
        "├─".bright_black(),
        "📝 Owner:".bright_yellow(), 
        owner.bright_white()
    );
    
    // Metadata
    if let Some(meta_str) = meta {
        println!("{}  {} {}", 
            "├─".bright_black(),
            "📊 Meta:".bright_magenta(), 
            format_data(meta_str)
        );
    } else {
        println!("{}  {} {}", 
            "├─".bright_black(),
            "📊 Meta:".bright_magenta(), 
            "<empty>".bright_black()
        );
    }
    
    // Payload
    if let Some(payload_str) = payload {
        println!("{}  {} {}", 
            "└─".bright_black(),
            "🔐 Payload:".bright_green(), 
            format_data(payload_str)
        );
    } else {
        println!("{}  {} {}", 
            "└─".bright_black(),
            "🔐 Payload:".bright_green(), 
            "<empty>".bright_black()
        );
    }
    
    // Children
    if !children.is_empty() {
        println!("\n{}  {} {}", 
            "   ".bright_black(),
            "👶 Children:".bright_blue(),
            format!("({} nodes)", children.len()).bright_black()
        );
        for (i, child_id) in children.iter().enumerate() {
            let prefix = if i == children.len() - 1 { "└─" } else { "├─" };
            println!("      {} {} {}", 
                prefix.bright_black(),
                "NodeId:".bright_white(),
                child_id.to_string().bright_cyan()
            );
        }
    } else {
        println!("\n{}  {} {}", 
            "   ".bright_black(),
            "👶 Children:".bright_blue(),
            "<none>".bright_black()
        );
    }
    
    println!();
}

fn format_data(data: &str) -> ColoredString {
    // Try to parse as JSON for pretty formatting
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
        serde_json::to_string_pretty(&json)
            .unwrap_or_else(|_| data.to_string())
            .bright_white()
    } else if data.starts_with("{") && data.contains("\"version\"") {
        // Encrypted data
        "[Encrypted]".bright_red().bold()
    } else if data.len() > 100 {
        // Long data, truncate
        format!("{}...", &data[..97]).bright_white()
    } else {
        data.bright_white()
    }
}

pub fn success(msg: &str) {
    println!("{} {}", "✅".green(), msg.green());
}

pub fn error(msg: &str) {
    eprintln!("{} {}", "❌".red(), msg.red());
}

pub fn info(msg: &str) {
    println!("{} {}", "ℹ️".blue(), msg.bright_blue());
}

pub fn warning(msg: &str) {
    println!("{} {}", "⚠️".yellow(), msg.yellow());
}

pub fn progress(msg: &str) {
    println!("{} {}", "🔄".cyan(), msg.cyan());
}
