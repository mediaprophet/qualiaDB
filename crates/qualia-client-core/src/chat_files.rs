//! Session-scoped chat file attachments — PDF/text extraction, image sharing, vision ingest.

use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};

use qualia_core_db::{q_hash, wal::WriteAheadLog, QualiaQuin};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::chat_session::{self, SessionKind};

const OBJECT_HASH_MASK: u64 = 0x0FFF_FFFF_FFFF_FFFF;
const MAX_EXTRACTED_CHARS: usize = 512_000;
const PREVIEW_CHARS: usize = 400;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FileVisibility {
    OwnerOnly,
    SessionParticipants,
    SpecificDids,
    PublicInSession,
}

impl FileVisibility {
    pub fn as_str(&self) -> &'static str {
        match self {
            FileVisibility::OwnerOnly => "owner_only",
            FileVisibility::SessionParticipants => "session_participants",
            FileVisibility::SpecificDids => "specific_dids",
            FileVisibility::PublicInSession => "public_in_session",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "owner_only" => Ok(FileVisibility::OwnerOnly),
            "session_participants" => Ok(FileVisibility::SessionParticipants),
            "specific_dids" => Ok(FileVisibility::SpecificDids),
            "public_in_session" => Ok(FileVisibility::PublicInSession),
            _ => Err(format!("unknown file visibility: {s}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatFileSharing {
    pub visibility: FileVisibility,
    pub allow_download: bool,
    pub allow_llm_context: bool,
    pub allow_relay_sync: bool,
    #[serde(default)]
    pub allowed_dids: Vec<String>,
    #[serde(default)]
    pub expires_at: Option<u64>,
}

impl Default for ChatFileSharing {
    fn default() -> Self {
        Self {
            visibility: FileVisibility::SessionParticipants,
            allow_download: true,
            allow_llm_context: true,
            allow_relay_sync: false,
            allowed_dids: vec![],
            expires_at: None,
        }
    }
}

pub fn default_sharing_for_session(kind: SessionKind) -> ChatFileSharing {
    match kind {
        SessionKind::Solo => ChatFileSharing {
            visibility: FileVisibility::OwnerOnly,
            allow_download: true,
            allow_llm_context: true,
            allow_relay_sync: false,
            allowed_dids: vec![],
            expires_at: None,
        },
        SessionKind::Group => ChatFileSharing::default(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfPageExtract {
    pub page_index: u32,
    pub text: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum MediaKind {
    #[default]
    Document,
    Image,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedDocument {
    pub media_kind: MediaKind,
    pub mime_type: String,
    pub extension: String,
    pub page_count: Option<u32>,
    pub image_width: Option<u32>,
    pub image_height: Option<u32>,
    pub full_text: String,
    pub pages: Vec<PdfPageExtract>,
    pub parse_status: String,
    pub parse_error: Option<String>,
    #[serde(default)]
    pub thumbnail_rel_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatFileRecord {
    pub file_id: String,
    pub original_name: String,
    pub mime_type: String,
    pub extension: String,
    pub sha256: String,
    pub byte_size: u64,
    pub page_count: Option<u32>,
    pub text_preview: String,
    pub storage_rel_path: String,
    pub text_rel_path: String,
    pub author_did: String,
    pub author_name: Option<String>,
    pub message_lamport: Option<u64>,
    pub attached_at: u64,
    pub sharing: ChatFileSharing,
    pub parse_status: String,
    pub parse_error: Option<String>,
    #[serde(default)]
    pub media_kind: MediaKind,
    #[serde(default)]
    pub image_width: Option<u32>,
    #[serde(default)]
    pub image_height: Option<u32>,
    #[serde(default)]
    pub thumbnail_rel_path: Option<String>,
    #[serde(default)]
    pub vision_lexicon_id: Option<String>,
    #[serde(default)]
    pub vision_facet: Option<String>,
    #[serde(default)]
    pub vision_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachChatFileResult {
    pub file: ChatFileRecord,
    pub message_lamport: u64,
}

fn files_index_path(storage_root: &Path, session_id: &str) -> PathBuf {
    chat_session::chats_dir(storage_root)
        .join(session_id)
        .join("files.jsonl")
}

fn files_dir(storage_root: &Path, session_id: &str) -> PathBuf {
    chat_session::chats_dir(storage_root)
        .join(session_id)
        .join("files")
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

fn extension_of(name: &str) -> String {
    Path::new(name)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase()
}

fn mime_for_extension(ext: &str) -> &'static str {
    match ext {
        "pdf" => "application/pdf",
        "txt" => "text/plain",
        "md" | "markdown" => "text/markdown",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        _ => "application/octet-stream",
    }
}

pub fn is_image_extension(ext: &str) -> bool {
    matches!(ext, "png" | "jpg" | "jpeg" | "webp" | "gif")
}

pub fn is_image_record(record: &ChatFileRecord) -> bool {
    record.media_kind == MediaKind::Image || is_image_extension(&record.extension)
}

pub fn parse_document_bytes(name: &str, bytes: &[u8]) -> ParsedDocument {
    parse_document_bytes_with_dir(name, bytes, None)
}

pub fn parse_document_bytes_with_dir(
    name: &str,
    bytes: &[u8],
    thumb_dir: Option<&Path>,
) -> ParsedDocument {
    let ext = extension_of(name);
    let mime_type = mime_for_extension(&ext).to_string();

    if is_image_extension(&ext) {
        return parse_image_bytes(bytes, mime_type, ext, thumb_dir);
    }

    if ext == "pdf" {
        return parse_pdf_bytes(bytes, mime_type, ext);
    }

    if ext == "txt" || ext == "md" || ext == "markdown" {
        let text = String::from_utf8_lossy(bytes).into_owned();
        let truncated = truncate_text(&text);
        return ParsedDocument {
            media_kind: MediaKind::Document,
            mime_type,
            extension: ext,
            page_count: None,
            image_width: None,
            image_height: None,
            full_text: truncated.clone(),
            pages: vec![],
            parse_status: "ok".to_string(),
            parse_error: None,
            thumbnail_rel_path: None,
        };
    }

    let ext_copy = ext.clone();
    ParsedDocument {
        media_kind: MediaKind::Document,
        mime_type,
        extension: ext,
        page_count: None,
        image_width: None,
        image_height: None,
        full_text: String::new(),
        pages: vec![],
        parse_status: "unsupported".to_string(),
        parse_error: Some(format!("Unsupported extension for chat file: {ext_copy}")),
        thumbnail_rel_path: None,
    }
}

fn parse_image_bytes(
    bytes: &[u8],
    mime_type: String,
    ext: String,
    thumb_dir: Option<&Path>,
) -> ParsedDocument {
    match image::load_from_memory(bytes) {
        Ok(img) => {
            let (w, h) = (img.width(), img.height());
            let thumb_rel = None;
            let _ = thumb_dir;
            let facet = format!(
                "image attachment {w}x{h} {mime_type} sha256:{}",
                &sha256_hex(bytes)[..16]
            );
            let full_text = format!(
                "[Image attachment]\nfilename_extension: {ext}\nmime: {mime_type}\ndimensions: {w}x{h}\nsha256: {}\nvision_facet: {facet}",
                sha256_hex(bytes)
            );
            ParsedDocument {
                media_kind: MediaKind::Image,
                mime_type,
                extension: ext,
                page_count: None,
                image_width: Some(w),
                image_height: Some(h),
                full_text,
                pages: vec![],
                parse_status: "ok".to_string(),
                parse_error: None,
                thumbnail_rel_path: thumb_rel,
            }
        }
        Err(e) => ParsedDocument {
            media_kind: MediaKind::Image,
            mime_type,
            extension: ext,
            page_count: None,
            image_width: None,
            image_height: None,
            full_text: String::new(),
            pages: vec![],
            parse_status: "failed".to_string(),
            parse_error: Some(format!("Image decode failed: {e}")),
            thumbnail_rel_path: None,
        },
    }
}

fn write_thumbnail(dir: &Path, file_id: &str, img: &image::DynamicImage) -> Option<String> {
    let name = format!("{file_id}_thumb.jpg");
    let path = dir.join(&name);
    let thumb = img.thumbnail(320, 320);
    thumb.save_with_format(&path, image::ImageFormat::Jpeg).ok()?;
    Some(format!("files/{name}"))
}

fn parse_pdf_bytes(bytes: &[u8], mime_type: String, ext: String) -> ParsedDocument {
    match pdf_extract::extract_text_from_mem_by_pages(bytes) {
        Ok(pages) => {
            let page_count = pages.len() as u32;
            let mut full = String::new();
            let mut page_extracts = Vec::with_capacity(pages.len());
            for (i, page_text) in pages.into_iter().enumerate() {
                if !full.is_empty() {
                    full.push_str("\n\n");
                }
                full.push_str(&format!("--- Page {} ---\n{}", i + 1, page_text));
                page_extracts.push(PdfPageExtract {
                    page_index: i as u32,
                    text: page_text,
                });
            }
            let truncated = truncate_text(&full);
            let status = if truncated.trim().is_empty() {
                "partial"
            } else {
                "ok"
            };
            ParsedDocument {
                media_kind: MediaKind::Document,
                mime_type,
                extension: ext,
                page_count: Some(page_count),
                image_width: None,
                image_height: None,
                full_text: truncated,
                pages: page_extracts,
                parse_status: status.to_string(),
                parse_error: if status == "partial" {
                    Some("PDF parsed but no extractable text (scanned image PDF)".to_string())
                } else {
                    None
                },
                thumbnail_rel_path: None,
            }
        }
        Err(e) => ParsedDocument {
            media_kind: MediaKind::Document,
            mime_type,
            extension: ext,
            page_count: None,
            image_width: None,
            image_height: None,
            full_text: String::new(),
            pages: vec![],
            parse_status: "failed".to_string(),
            parse_error: Some(format!("{e}")),
            thumbnail_rel_path: None,
        },
    }
}

fn try_vision_bind(storage_root: &Path, source_path: &Path) -> (Option<String>, Option<String>, String) {
    let active = crate::api::load_active_model_record_from_disk();
    match crate::vision_ingest::ingest_image_with_active_record(
        storage_root,
        active,
        source_path,
        "ChatShare",
    ) {
        Ok(result) => (
            Some(result.lexicon_id),
            Some(result.facet),
            "ok".to_string(),
        ),
        Err(e) => (None, None, format!("skipped:{e}")),
    }
}

fn truncate_text(text: &str) -> String {
    if text.len() <= MAX_EXTRACTED_CHARS {
        return text.to_string();
    }
    let mut end = MAX_EXTRACTED_CHARS;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &text[..end])
}

fn preview_text(text: &str) -> String {
    let flat = text.replace('\n', " ");
    if flat.chars().count() <= PREVIEW_CHARS {
        return flat;
    }
    flat.chars().take(PREVIEW_CHARS).collect::<String>() + "…"
}

fn load_all_records(storage_root: &Path, session_id: &str) -> Result<Vec<ChatFileRecord>, String> {
    let path = files_index_path(storage_root, session_id);
    if !path.is_file() {
        return Ok(vec![]);
    }
    let file = File::open(path).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);
    let mut out = Vec::new();
    for line in reader.lines() {
        let line = line.map_err(|e| e.to_string())?;
        if line.trim().is_empty() {
            continue;
        }
        out.push(serde_json::from_str(&line).map_err(|e| e.to_string())?);
    }
    Ok(out)
}

fn write_all_records(
    storage_root: &Path,
    session_id: &str,
    records: &[ChatFileRecord],
) -> Result<(), String> {
    let path = files_index_path(storage_root, session_id);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    for r in records {
        writeln!(file, "{}", serde_json::to_string(r).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn session_participant_dids(storage_root: &Path, session_id: &str) -> Vec<String> {
    let Ok(session) = chat_session::load_session(storage_root, session_id) else {
        return vec![];
    };
    session
        .environment
        .participants
        .iter()
        .map(|p| p.did.clone())
        .collect()
}

pub fn can_view_file(
    record: &ChatFileRecord,
    viewer_did: &str,
    participant_dids: &[String],
) -> bool {
    if record.sharing.expires_at.is_some_and(|exp| unix_now() > exp) {
        return false;
    }
    if record.author_did == viewer_did {
        return true;
    }
    match record.sharing.visibility {
        FileVisibility::OwnerOnly => false,
        FileVisibility::SessionParticipants | FileVisibility::PublicInSession => {
            participant_dids.iter().any(|d| d == viewer_did)
                || participant_dids.is_empty()
        }
        FileVisibility::SpecificDids => record
            .sharing
            .allowed_dids
            .iter()
            .any(|d| d == viewer_did),
    }
}

pub fn can_use_in_llm_context(record: &ChatFileRecord, viewer_did: &str, participants: &[String]) -> bool {
    record.sharing.allow_llm_context && can_view_file(record, viewer_did, participants)
}

pub fn attach_chat_file(
    storage_root: &Path,
    session_id: &str,
    source_path: &Path,
    sharing: ChatFileSharing,
) -> Result<AttachChatFileResult, String> {
    if !source_path.is_file() {
        return Err(format!("File not found: {}", source_path.display()));
    }

    let ext = extension_of(
        source_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(""),
    );
    if ext != "pdf"
        && ext != "txt"
        && ext != "md"
        && ext != "markdown"
        && !is_image_extension(&ext)
    {
        return Err(format!(
            "Unsupported file type '.{ext}' — attach PDF, TXT, Markdown, or image (PNG/JPEG/WebP/GIF)"
        ));
    }

    let mut bytes = Vec::new();
    File::open(source_path)
        .and_then(|mut f| f.read_to_end(&mut bytes))
        .map_err(|e| e.to_string())?;

    let sha = sha256_hex(&bytes);
    let file_id = format!("{:016x}", q_hash(&format!("chatfile:{session_id}:{sha}")));
    let original_name = source_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("attachment")
        .to_string();

    let dir = files_dir(storage_root, session_id);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let mut parsed = parse_document_bytes(&original_name, &bytes);
    if parsed.media_kind == MediaKind::Image {
        if let Ok(img) = image::load_from_memory(&bytes) {
            parsed.thumbnail_rel_path = write_thumbnail(&dir, &file_id, &img);
        }
    }

    let (vision_lexicon_id, vision_facet, vision_status) = if parsed.media_kind == MediaKind::Image {
        let (lex, facet, status) = try_vision_bind(storage_root, source_path);
        if let Some(ref f) = facet {
            parsed.full_text.push_str(&format!("\nvision_ingest: {f}"));
        }
        (lex, facet, status)
    } else {
        (None, None, String::new())
    };

    let bin_name = format!("{file_id}.bin");
    let txt_name = format!("{file_id}.txt");
    fs::write(dir.join(&bin_name), &bytes).map_err(|e| e.to_string())?;
    fs::write(dir.join(&txt_name), &parsed.full_text).map_err(|e| e.to_string())?;

    let profile = crate::user_profile::load_profile();
    let author_name = profile
        .sharing
        .share_display_name
        .then(|| profile.display_name.clone());

    let text_preview = if parsed.media_kind == MediaKind::Image {
        match (parsed.image_width, parsed.image_height) {
            (Some(w), Some(h)) => format!("{w}×{h} {}", parsed.mime_type),
            _ => parsed.mime_type.clone(),
        }
    } else {
        preview_text(&parsed.full_text)
    };

    let record = ChatFileRecord {
        file_id: file_id.clone(),
        original_name: original_name.clone(),
        mime_type: parsed.mime_type,
        extension: parsed.extension,
        sha256: sha,
        byte_size: bytes.len() as u64,
        page_count: parsed.page_count,
        text_preview,
        storage_rel_path: format!("files/{bin_name}"),
        text_rel_path: format!("files/{txt_name}"),
        author_did: profile.public_did.clone(),
        author_name,
        message_lamport: None,
        attached_at: unix_now(),
        sharing,
        parse_status: parsed.parse_status,
        parse_error: parsed.parse_error,
        media_kind: parsed.media_kind,
        image_width: parsed.image_width,
        image_height: parsed.image_height,
        thumbnail_rel_path: parsed.thumbnail_rel_path.clone(),
        vision_lexicon_id: vision_lexicon_id.clone(),
        vision_facet: vision_facet.clone(),
        vision_status: if vision_status.is_empty() {
            None
        } else {
            Some(vision_status)
        },
    };

    let size_note = if record.media_kind == MediaKind::Image {
        match (record.image_width, record.image_height) {
            (Some(w), Some(h)) => format!(" ({w}×{h})"),
            _ => String::new(),
        }
    } else {
        record
            .page_count
            .map(|n| format!(" ({n} pages)"))
            .unwrap_or_default()
    };
    let icon = if record.media_kind == MediaKind::Image {
        "🖼️"
    } else {
        "📎"
    };
    let msg_content = format!(
        "{icon} Attached {}: {original_name}{size_note} [{}]",
        if record.media_kind == MediaKind::Image {
            "image"
        } else {
            "file"
        },
        record.sharing.visibility.as_str()
    );
    let lamport = chat_session::append_message_with_author(
        storage_root,
        session_id,
        chat_session::Role::User,
        &msg_content,
        None,
        Some("chat_file".to_string()),
        Some(profile.public_did.clone()),
        record.author_name.clone(),
        None,
    )
    .map_err(|e| e.to_string())?;

    let mut record = record;
    record.message_lamport = Some(lamport);

    let mut records = load_all_records(storage_root, session_id)?;
    records.push(record.clone());
    write_all_records(storage_root, session_id, &records)?;

    append_file_wal_quin(storage_root, session_id, &record)?;

    Ok(AttachChatFileResult {
        file: record,
        message_lamport: lamport,
    })
}

fn append_file_wal_quin(
    storage_root: &Path,
    session_id: &str,
    record: &ChatFileRecord,
) -> Result<(), String> {
    let wal_path = chat_session::chats_dir(storage_root)
        .join(session_id)
        .join("chat.wal");
    if !wal_path.is_file() {
        return Ok(());
    }
    let subject = q_hash(&format!("chat:session:{session_id}"));
    let predicate = q_hash("chat:hasFile");
    let object = u64::from_str_radix(&record.file_id, 16).unwrap_or(0) & OBJECT_HASH_MASK;
    let context = q_hash(&record.author_did);
    let metadata = (record.byte_size.min(0x1FFF_FFFF)) << 32;
    let parity = subject ^ predicate ^ object ^ context ^ metadata;
    let quin = QualiaQuin {
        subject,
        predicate,
        object,
        context,
        metadata,
        parity,
    };
    if let Ok(mut wal) = WriteAheadLog::open(&wal_path) {
        wal.append_mutation(&quin).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn set_chat_file_sharing(
    storage_root: &Path,
    session_id: &str,
    file_id: &str,
    sharing: ChatFileSharing,
) -> Result<ChatFileRecord, String> {
    let profile = crate::user_profile::load_profile();
    let mut records = load_all_records(storage_root, session_id)?;
    let idx = records
        .iter()
        .position(|r| r.file_id == file_id)
        .ok_or_else(|| format!("Chat file not found: {file_id}"))?;
    if records[idx].author_did != profile.public_did {
        return Err("Only the file owner can change sharing permissions".to_string());
    }
    records[idx].sharing = sharing;
    let updated = records[idx].clone();
    write_all_records(storage_root, session_id, &records)?;
    Ok(updated)
}

pub fn list_chat_files(
    storage_root: &Path,
    session_id: &str,
    viewer_did: Option<&str>,
) -> Result<Vec<ChatFileRecord>, String> {
    let viewer = viewer_did
        .map(|s| s.to_string())
        .unwrap_or_else(|| crate::user_profile::load_profile().public_did);
    let participants = session_participant_dids(storage_root, session_id);
    let records = load_all_records(storage_root, session_id)?;
    Ok(records
        .into_iter()
        .filter(|r| can_view_file(r, &viewer, &participants))
        .collect())
}

pub fn read_file_text(
    storage_root: &Path,
    session_id: &str,
    file_id: &str,
    viewer_did: Option<&str>,
) -> Result<String, String> {
    let viewer = viewer_did
        .map(|s| s.to_string())
        .unwrap_or_else(|| crate::user_profile::load_profile().public_did);
    let participants = session_participant_dids(storage_root, session_id);
    let records = load_all_records(storage_root, session_id)?;
    let record = records
        .iter()
        .find(|r| r.file_id == file_id)
        .ok_or_else(|| format!("Chat file not found: {file_id}"))?;
    if !can_view_file(record, &viewer, &participants) {
        return Err("You do not have permission to view this file".to_string());
    }
    let text_path = chat_session::chats_dir(storage_root)
        .join(session_id)
        .join(&record.text_rel_path);
    fs::read_to_string(text_path).map_err(|e| e.to_string())
}

pub fn build_chat_files_context_block(
    storage_root: &Path,
    session_id: &str,
    max_chars: usize,
) -> String {
    let profile = crate::user_profile::load_profile();
    let participants = session_participant_dids(storage_root, session_id);
    let Ok(files) = list_chat_files(storage_root, session_id, Some(&profile.public_did)) else {
        return String::new();
    };

    let mut lines = vec!["[Chat attached files]".to_string()];
    let mut used = 0usize;

    for f in &files {
        if !can_use_in_llm_context(f, &profile.public_did, &participants) {
            continue;
        }
        let dim = match (f.image_width, f.image_height) {
            (Some(w), Some(h)) => format!(", {w}x{h}"),
            _ => String::new(),
        };
        let header = format!(
            "- {} ({}{}{} bytes, visibility={})",
            f.original_name,
            f.mime_type,
            dim,
            f.byte_size,
            f.sharing.visibility.as_str()
        );
        used += header.len();
        lines.push(header);

        if let Some(ref facet) = f.vision_facet {
            let line = format!("  vision_facet: {facet}");
            used += line.len();
            lines.push(line);
        }

        if used >= max_chars {
            lines.push("  … (truncated)".to_string());
            break;
        }

        if is_image_record(f) {
            lines.push("  note: multimodal image — use active VLM mmproj when vision_status=ok".to_string());
            continue;
        }

        if let Ok(text) = read_file_text(storage_root, session_id, &f.file_id, Some(&profile.public_did))
        {
            let budget = max_chars.saturating_sub(used);
            let excerpt = if text.len() <= budget {
                text
            } else {
                let mut end = budget;
                while end > 0 && !text.is_char_boundary(end) {
                    end -= 1;
                }
                format!("{}…", &text[..end])
            };
            used += excerpt.len();
            lines.push(format!("  excerpt: {excerpt}"));
        }
    }

    if lines.len() == 1 {
        return String::new();
    }
    lines.join("\n")
}

pub fn resolve_chat_file_path(
    storage_root: &Path,
    session_id: &str,
    file_id: &str,
    variant: &str,
    viewer_did: Option<&str>,
) -> Result<PathBuf, String> {
    let viewer = viewer_did
        .map(|s| s.to_string())
        .unwrap_or_else(|| crate::user_profile::load_profile().public_did);
    let participants = session_participant_dids(storage_root, session_id);
    let records = load_all_records(storage_root, session_id)?;
    let record = records
        .iter()
        .find(|r| r.file_id == file_id)
        .ok_or_else(|| format!("Chat file not found: {file_id}"))?;
    if !can_view_file(record, &viewer, &participants) {
        return Err("You do not have permission to view this file".to_string());
    }
    let rel = match variant {
        "thumbnail" => record
            .thumbnail_rel_path
            .as_deref()
            .unwrap_or(&record.storage_rel_path),
        _ => &record.storage_rel_path,
    };
    Ok(chat_session::chats_dir(storage_root)
        .join(session_id)
        .join(rel))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn default_sharing_differs_by_session_kind() {
        let solo = default_sharing_for_session(SessionKind::Solo);
        assert_eq!(solo.visibility, FileVisibility::OwnerOnly);
        let group = default_sharing_for_session(SessionKind::Group);
        assert_eq!(group.visibility, FileVisibility::SessionParticipants);
    }

    #[test]
    fn parse_png_image() {
        let img = image::RgbaImage::from_pixel(2, 2, image::Rgba([10, 20, 30, 255]));
        let dyn_img = image::DynamicImage::ImageRgba8(img);
        let mut buf = Vec::new();
        dyn_img
            .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
            .expect("encode png");
        let parsed = parse_document_bytes("snap.png", &buf);
        assert_eq!(parsed.media_kind, MediaKind::Image);
        assert_eq!(parsed.image_width, Some(2));
        assert_eq!(parsed.parse_status, "ok");
    }

    #[test]
    fn parse_txt_document() {
        let parsed = parse_document_bytes("notes.txt", b"hello chat files");
        assert_eq!(parsed.parse_status, "ok");
        assert!(parsed.full_text.contains("hello"));
    }

    #[test]
    fn sharing_permission_gate() {
        let record = ChatFileRecord {
            file_id: "abc".to_string(),
            original_name: "x.pdf".to_string(),
            mime_type: "application/pdf".to_string(),
            extension: "pdf".to_string(),
            sha256: "00".to_string(),
            byte_size: 10,
            page_count: Some(1),
            text_preview: "p".to_string(),
            storage_rel_path: "files/a.bin".to_string(),
            text_rel_path: "files/a.txt".to_string(),
            author_did: "did:owner".to_string(),
            author_name: None,
            message_lamport: Some(1),
            attached_at: 0,
            sharing: ChatFileSharing {
                visibility: FileVisibility::OwnerOnly,
                allow_download: false,
                allow_llm_context: true,
                allow_relay_sync: false,
                allowed_dids: vec![],
                expires_at: None,
            },
            parse_status: "ok".to_string(),
            parse_error: None,
            media_kind: MediaKind::Document,
            image_width: None,
            image_height: None,
            thumbnail_rel_path: None,
            vision_lexicon_id: None,
            vision_facet: None,
            vision_status: None,
        };
        assert!(can_view_file(&record, "did:owner", &[]));
        assert!(!can_view_file(&record, "did:other", &[]));
    }

    #[test]
    fn attach_and_list_round_trip() {
        let mut storage = env::temp_dir();
        storage.push(format!("qualia-chat-files-{}", rand::random::<u32>()));
        let session_id = chat_session::create_session(&storage, Some("Files test".to_string()), None)
            .expect("create session");

        let src = storage.join("sample.md");
        fs::write(&src, "# Title\n\nBody text for chat.").unwrap();

        let sharing = default_sharing_for_session(SessionKind::Group);
        let attached = attach_chat_file(&storage, &session_id, &src, sharing).expect("attach");
        assert_eq!(attached.file.extension, "md");
        assert!(attached.message_lamport > 0);

        let owner_did = attached.file.author_did.clone();
        let listed = list_chat_files(&storage, &session_id, Some(&owner_did)).unwrap();
        assert_eq!(listed.len(), 1);

        let updated = set_chat_file_sharing(
            &storage,
            &session_id,
            &attached.file.file_id,
            ChatFileSharing {
                visibility: FileVisibility::SpecificDids,
                allow_download: true,
                allow_llm_context: false,
                allow_relay_sync: false,
                allowed_dids: vec!["did:friend".to_string()],
                expires_at: None,
            },
        )
        .expect("set sharing");
        assert_eq!(updated.sharing.visibility, FileVisibility::SpecificDids);
    }
}
