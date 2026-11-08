//! Asset / attachment command adapters over [`stratum_app::asset`].

pub use stratum_app::asset::AssetBytesDto;

use stratum_app::StratumService;

#[tauri::command]
pub async fn import_asset_path(
    path: String,
    dest_name: Option<String>,
    state: tauri::State<'_, StratumService>,
) -> Result<pkm_block::assets::AssetRef, String> {
    state.import_asset_path(path, dest_name).await
}

#[tauri::command]
pub async fn import_asset_bytes(
    data_base64: String,
    original_name: String,
    mime: Option<String>,
    dest_name: Option<String>,
    state: tauri::State<'_, StratumService>,
) -> Result<pkm_block::assets::AssetRef, String> {
    state
        .import_asset_bytes(data_base64, original_name, mime, dest_name)
        .await
}

#[tauri::command]
pub async fn read_asset(
    ref_path: String,
    state: tauri::State<'_, StratumService>,
) -> Result<AssetBytesDto, String> {
    state.read_asset(ref_path).await
}

#[tauri::command]
pub async fn delete_asset(
    ref_path: String,
    state: tauri::State<'_, StratumService>,
) -> Result<(), String> {
    state.delete_asset(ref_path).await
}

#[tauri::command]
pub async fn list_asset_refs(
    state: tauri::State<'_, StratumService>,
) -> Result<Vec<pkm_block::assets::AssetRef>, String> {
    state.list_asset_refs().await
}

#[tauri::command]
pub async fn get_page_asset_refs(
    page_path: String,
    state: tauri::State<'_, StratumService>,
) -> Result<Vec<pkm_block::assets::AssetRef>, String> {
    state.get_page_asset_refs(page_path).await
}
