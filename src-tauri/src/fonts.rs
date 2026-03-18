use font_kit::source::SystemSource;

/// 获取系统字体族列表，排序去重后返回
#[tauri::command]
pub fn get_system_fonts() -> Result<Vec<String>, String> {
    let source = SystemSource::new();
    let mut families = source
        .all_families()
        .map_err(|e| format!("读取系统字体失败: {}", e))?;
    families.sort_unstable_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
    families.dedup();
    Ok(families)
}
