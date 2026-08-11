// AxAIHub 插件打包工具 (CLI)
// 支持打包 .axext (开源 ZIP) 和 .axex (AES-256-GCM 加密 + 可选 Ed25519 作者签名)
//
// 本工具的输出格式与手机端 AxExt 打包工具 (App 内) 字节级对齐, 互相可导入:
//   .axex v2: AXEX[version=2][plugin_id_len][plugin_id][pub_key_len(2)][pub_key][sig_len(2)][sig][nonce(12)][ciphertext]
//   .axex v1: AXEX[version=1][plugin_id_len][plugin_id][nonce(12)][ciphertext]   (无签名段)
//
// 元数据统一使用 .ns (NexusScript) 格式, 回退 manifest.json。
// 默认加密密钥与 App 内置密钥一致: AxAIHub-Default-Dev-Key!
// 签名使用开发者自己的 Ed25519 私钥, 没有"默认签名密钥" —— 作者身份由开发者自备密钥保证。
//
// 使用:
//   axbuild pack ./my-plugin my-plugin.axext
//   axbuild pack ./my-plugin my-plugin.axex --encrypt
//   axbuild pack ./my-plugin my-plugin.axex --encrypt --master-key "custom-key"
//   axbuild pack ./my-plugin my-plugin.axex --encrypt --private-key "<base64 私钥>"
//   axbuild keypair
//   axbuild new ./my-plugin --id com.example.myplugin --name "我的插件" --author "作者名"
//   axbuild info my-plugin.axex
//   axbuild info my-plugin.axex --master-key "custom-key"

use std::fs;
use std::path::Path;
use std::io::{Read, Write};
use std::collections::HashMap;
use clap::{Parser, Subcommand};
use serde_json::Value;
use aes_gcm::{Aes256Gcm, Key, Nonce};
use aes_gcm::aead::{Aead, KeyInit};
use pbkdf2::pbkdf2_hmac;
use sha2::{Sha256, Digest};
use rand::RngCore;
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use ed25519_compact::*;

/// AxAIHub 插件打包工具
#[derive(Parser)]
#[command(name = "axbuild", version = "1.0.0", about = "AxAIHub 插件打包工具 (与 App 格式对齐)")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 打包插件目录为 .axext / .axex 文件
    Pack {
        /// 源目录 (包含 .ns + index.html 等)
        src_dir: String,

        /// 输出路径 (.axext 或 .axex 后缀)
        output: String,

        /// 是否加密打包为 .axex (AES-256-GCM)
        #[arg(long, short = 'e')]
        encrypt: bool,

        /// 自定义加密主密钥, 省略则使用内置默认密钥
        #[arg(long)]
        master_key: Option<String>,

        /// 可选 Ed25519 私钥 (base64), 提供则对插件进行作者签名 (生成 v2 包)
        #[arg(long)]
        private_key: Option<String>,
    },

    /// 生成插件模板 (.ns + index.html)
    New {
        /// 目标目录
        #[arg(default_value = ".")]
        dest_dir: String,

        /// 插件 ID (反向域名格式, 全局唯一)
        #[arg(long)]
        id: String,

        /// 插件名称
        #[arg(long)]
        name: String,

        /// 作者
        #[arg(long)]
        author: String,
    },

    /// 生成 Ed25519 密钥对 (用于 .axex 作者签名)
    Keypair,

    /// 查看插件包信息 (.axext / .axex)
    Info {
        /// 插件包路径 (.axext 或 .axex)
        path: String,

        /// .axex 解密密钥, 省略则用内置默认密钥
        #[arg(long)]
        master_key: Option<String>,
    },
}

/// 默认加密主密钥, 与 App 内置密钥一致
/// 注意: 这只是 AES 加密层的默认密钥。作者签名使用的是开发者自己的 Ed25519 私钥, 没有默认值。
const DEFAULT_MASTER_KEY: &str = "AxAIHub-Default-Dev-Key!";

/// 打包时排除的文件/目录名 (不区分大小写), 与 App builder.rs 保持一致
const EXCLUDED_NAMES: &[&str] = &[
    ".git", ".svn", ".hg", ".DS_Store", "Thumbs.db",
    "node_modules", ".gitignore", ".gitkeep", ".gitattributes",
    ".editorconfig", ".eslintrc", ".prettierrc",
    "package-lock.json", "yarn.lock", "pnpm-lock.yaml",
    "Cargo.lock", ".npmrc", ".env", ".env.local",
];

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Pack { src_dir, output, encrypt, master_key, private_key } => {
            if encrypt || output.ends_with(".axex") {
                cmd_pack_axex(&src_dir, &output, &resolve_master_key(master_key.as_deref()), private_key.as_deref());
            } else {
                cmd_pack_axext(&src_dir, &output);
            }
        }
        Commands::New { dest_dir, id, name, author } => {
            cmd_gen_template(&dest_dir, &id, &name, &author);
        }
        Commands::Keypair => {
            cmd_keypair();
        }
        Commands::Info { path, master_key } => {
            cmd_info(&path, &resolve_master_key(master_key.as_deref()));
        }
    }
}

/// 获取主密钥: 优先命令行参数, 其次环境变量 AXBUILD_MASTER_KEY, 最后内置默认密钥
fn resolve_master_key(cli_key: Option<&str>) -> String {
    if let Some(k) = cli_key {
        if !k.is_empty() {
            return k.to_string();
        }
    }
    if let Ok(env_key) = std::env::var("AXBUILD_MASTER_KEY") {
        if !env_key.is_empty() {
            return env_key;
        }
    }
    DEFAULT_MASTER_KEY.to_string()
}

// ============================================================================
// .ns (NexusScript) 解析: 仅提取顶层标量字段 (id/type/entry 等)
// 与 App 的 src-tauri/src/plugin/ns.rs 同源思路, 这里做最小实现以满足打包需求
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Word(String),
    Open,     // {
    Close,    // }
    ArrOpen,  // [
    ArrClose, // ]
}

/// 极简 tokenizer: 字符串整体保留(含引号), { } [ ] 各自成 token, # 仅在非字符串内触发注释
fn tokenize_ns(input: &str) -> Vec<Tok> {
    let mut toks: Vec<Tok> = Vec::new();
    let mut buf = String::new();
    let mut chars = input.chars().peekable();
    let mut in_str = false;

    while let Some(&c) = chars.peek() {
        if in_str {
            if c == '\\' {
                buf.push(c);
                chars.next();
                if let Some(&n) = chars.peek() {
                    buf.push(n);
                    chars.next();
                }
            } else if c == '"' {
                buf.push(c);
                chars.next();
                in_str = false;
                flush_word(&mut buf, &mut toks);
            } else {
                buf.push(c);
                chars.next();
            }
        } else {
            match c {
                '"' => {
                    flush_word(&mut buf, &mut toks);
                    buf.push(c);
                    in_str = true;
                    chars.next();
                }
                '{' => { flush_word(&mut buf, &mut toks); toks.push(Tok::Open); chars.next(); }
                '}' => { flush_word(&mut buf, &mut toks); toks.push(Tok::Close); chars.next(); }
                '[' => { flush_word(&mut buf, &mut toks); toks.push(Tok::ArrOpen); chars.next(); }
                ']' => { flush_word(&mut buf, &mut toks); toks.push(Tok::ArrClose); chars.next(); }
                '#' => {
                    flush_word(&mut buf, &mut toks);
                    while let Some(&nc) = chars.peek() {
                        if nc == '\n' { break; }
                        chars.next();
                    }
                }
                _ if c.is_whitespace() => { flush_word(&mut buf, &mut toks); chars.next(); }
                _ => { buf.push(c); chars.next(); }
            }
        }
    }
    flush_word(&mut buf, &mut toks);
    toks
}

fn flush_word(buf: &mut String, toks: &mut Vec<Tok>) {
    if !buf.is_empty() {
        toks.push(Tok::Word(std::mem::take(buf)));
    }
}

/// 反转 Rust 风格转义 (与 App ns.rs 的 unescape_ns 一致)
fn unescape_ns(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some(other) => { out.push('\\'); out.push(other); }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// 提取 .ns 顶层标量字段, 返回 key -> value 映射
/// 仅收集顶层 (根对象直接子级) 的标量值; 嵌套对象/数组被跳过
fn ns_root_scalars(content: &str) -> HashMap<String, String> {
    let toks = tokenize_ns(content);
    let mut map = HashMap::new();
    let mut i = 0;
    // 跳过最外层 { (如果有)
    if toks.get(0) == Some(&Tok::Open) {
        i = 1;
    }
    while i < toks.len() {
        match &toks[i] {
            Tok::Close => break,
            Tok::Word(k) => {
                let key = k.clone();
                i += 1;
                if i >= toks.len() {
                    break;
                }
                match &toks[i] {
                    Tok::Open | Tok::ArrOpen => {
                        // 跳过嵌套块
                        let mut depth = 1;
                        i += 1;
                        while i < toks.len() && depth > 0 {
                            match &toks[i] {
                                Tok::Open | Tok::ArrOpen => depth += 1,
                                Tok::Close | Tok::ArrClose => depth -= 1,
                                _ => {}
                            }
                            i += 1;
                        }
                    }
                    Tok::Word(w) => {
                        map.insert(key, scalar_str(w));
                        i += 1;
                    }
                    Tok::Close => break,
                    _ => { i += 1; }
                }
            }
            _ => { i += 1; }
        }
    }
    map
}

fn scalar_str(w: &str) -> String {
    if w.starts_with('"') && w.ends_with('"') && w.len() >= 2 {
        unescape_ns(&w[1..w.len() - 1])
    } else if w == "true" || w == "false" {
        w.to_string()
    } else if w == "null" || w == "none" {
        String::new()
    } else {
        w.to_string()
    }
}

/// 读取插件元数据 (.ns 优先, manifest.json 回退), 返回 (id, type, entry)
fn read_plugin_meta(dir: &Path) -> Result<(String, String, Option<String>), String> {
    let mut map: HashMap<String, String> = HashMap::new();

    // 优先 .ns 文件
    let ns_files: Vec<_> = fs::read_dir(dir)
        .map_err(|e| format!("读取目录失败: {}", e))?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "ns").unwrap_or(false))
        .collect();
    if let Some(ns_entry) = ns_files.first() {
        let content = fs::read_to_string(ns_entry.path())
            .map_err(|e| format!("读取 .ns 文件失败: {}", e))?;
        map = ns_root_scalars(&content);
    } else {
        // 回退 manifest.json
        let mf = dir.join("manifest.json");
        if mf.exists() {
            let content = fs::read_to_string(&mf)
                .map_err(|e| format!("读取 manifest.json 失败: {}", e))?;
            let v: Value = serde_json::from_str(&content)
                .map_err(|e| format!("解析 manifest.json 失败: {}", e))?;
            if let Some(obj) = v.as_object() {
                for (k, val) in obj {
                    if let Some(s) = val.as_str() {
                        map.insert(k.clone(), s.to_string());
                    }
                }
            }
        }
    }

    if map.is_empty() {
        return Err("找不到插件元数据文件 (.ns 或 manifest.json)".to_string());
    }
    let id = map.get("id").cloned().filter(|s| !s.is_empty())
        .ok_or("插件元数据缺少 id 字段")?;
    let type_ = map.get("type").cloned().unwrap_or_else(|| "local".to_string());
    let entry = map.get("entry").cloned().filter(|s| !s.is_empty());
    Ok((id, type_, entry))
}

/// 计算公钥指纹 (SHA256 前 8 字节, hex)
fn fingerprint(pub_key: &[u8]) -> String {
    let hash = Sha256::digest(pub_key);
    hex::encode(&hash[..8])
}

/// 递归收集目录文件, 排除垃圾文件, 返回 (相对路径, 绝对路径)
fn collect_files(dir: &Path, current: &Path, entries: &mut Vec<(String, String)>) -> Result<(), String> {
    for entry in fs::read_dir(current).map_err(|e| format!("读取目录失败: {}", e))? {
        let entry = entry.map_err(|e| format!("读取条目失败: {}", e))?;
        let path = entry.path();
        let file_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        if should_exclude(&file_name) {
            continue;
        }
        if path.is_dir() {
            collect_files(dir, &path, entries)?;
        } else if path.is_file() {
            let rel = path.strip_prefix(dir)
                .map_err(|e| format!("路径解析失败: {}", e))?
                .to_string_lossy()
                .to_string();
            entries.push((rel, path.to_string_lossy().to_string()));
        }
    }
    Ok(())
}

fn should_exclude(name: &str) -> bool {
    let lower = name.to_lowercase();
    EXCLUDED_NAMES.iter().any(|e| lower == *e || lower.ends_with(e))
}

// ============================================================================
// 打包 .axext (ZIP)
// ============================================================================
fn cmd_pack_axext(src_dir: &str, output_path: &str) {
    let dir = Path::new(src_dir);
    if !dir.is_dir() {
        eprintln!("错误: 源路径不是目录: {}", src_dir);
        std::process::exit(1);
    }

    // 校验元数据 (.ns 优先, 回退 manifest.json)
    match read_plugin_meta(dir) {
        Ok((id, _, _)) => { println!("元数据: 插件 ID = {}", id); }
        Err(e) => { eprintln!("错误: {}", e); std::process::exit(1); }
    }

    let file = match fs::File::create(output_path) {
        Ok(f) => f,
        Err(e) => { eprintln!("错误: 创建文件失败: {}", e); std::process::exit(1); }
    };
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    let mut entries = Vec::new();
    if let Err(e) = collect_files(dir, dir, &mut entries) {
        eprintln!("错误: {}", e); std::process::exit(1);
    }
    for (rel_path, abs_path) in &entries {
        if let Err(e) = zip.start_file(rel_path, options) {
            eprintln!("错误: 写入 ZIP 条目失败: {}", e);
            std::process::exit(1);
        }
        let mut f = match fs::File::open(abs_path) {
            Ok(f) => f,
            Err(e) => { eprintln!("错误: 打开文件失败: {}", e); std::process::exit(1); }
        };
        if let Err(e) = std::io::copy(&mut f, &mut zip) {
            eprintln!("错误: 写入数据失败: {}", e);
            std::process::exit(1);
        }
    }
    if let Err(e) = zip.finish() {
        eprintln!("错误: 完成 ZIP 失败: {}", e);
        std::process::exit(1);
    }
    println!("成功: 已生成 {} ({} 个文件)", output_path, entries.len());
}

// ============================================================================
// 打包 .axex (AES-256-GCM + 可选 Ed25519 签名)
// 格式与 App builder.rs 字节级对齐:
//   AXEX[version][plugin_id_len(1)][plugin_id]
//   [v2: pub_key_len(2 LE)][pub_key][sig_len(2 LE)][sig]
//   [nonce(12)][ciphertext]
// ============================================================================
fn cmd_pack_axex(src_dir: &str, output_path: &str, master_key: &str, private_key: Option<&str>) {
    let dir = Path::new(src_dir);
    if !dir.is_dir() {
        eprintln!("错误: 源路径不是目录: {}", src_dir);
        std::process::exit(1);
    }

    // 读取元数据 (.ns 优先)
    let (plugin_id, plugin_type, entry) = match read_plugin_meta(dir) {
        Ok(m) => m,
        Err(e) => { eprintln!("错误: {}", e); std::process::exit(1); }
    };
    if plugin_type != "remote" {
        if let Some(entry_file) = entry.as_ref() {
            let entry_path = dir.join(entry_file);
            if !entry_path.exists() {
                eprintln!("错误: 缺少入口文件: {}", entry_file);
                std::process::exit(1);
            }
        }
    }

    // 1. 打包为 ZIP 到内存 (排除垃圾文件)
    let mut file_count = 0u32;
    let mut zip_buf = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut zip_buf);
        let options = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        let mut entries = Vec::new();
        if let Err(e) = collect_files(dir, dir, &mut entries) {
            eprintln!("错误: {}", e);
            std::process::exit(1);
        }
        for (rel_path, abs_path) in &entries {
            if let Err(e) = zip.start_file(rel_path, options) {
                eprintln!("错误: 写入 ZIP 条目失败: {}", e);
                std::process::exit(1);
            }
            let mut f = match fs::File::open(abs_path) {
                Ok(f) => f,
                Err(e) => { eprintln!("错误: 打开文件失败: {}", e); std::process::exit(1); }
            };
            if let Err(e) = std::io::copy(&mut f, &mut zip) {
                eprintln!("错误: 写入数据失败: {}", e);
                std::process::exit(1);
            }
            file_count += 1;
        }
        if let Err(e) = zip.finish() {
            eprintln!("错误: 完成 ZIP 失败: {}", e);
            std::process::exit(1);
        }
    }
    let zip_data = zip_buf.into_inner();

    // 2. 加密密钥材料 (默认密钥或自定义)
    let key_material = if master_key.is_empty() { DEFAULT_MASTER_KEY } else { master_key };
    if key_material.len() < 8 {
        eprintln!("错误: 密钥长度至少 8 字节");
        std::process::exit(1);
    }
    let is_default_key = master_key.is_empty() || master_key == DEFAULT_MASTER_KEY;

    // 3. 派生密钥: PBKDF2-SHA256(key_material, plugin_id, 100000, 32)
    let mut derived_key = [0u8; 32];
    pbkdf2_hmac::<Sha256>(key_material.as_bytes(), plugin_id.as_bytes(), 100_000, &mut derived_key);

    // 4. Ed25519 签名 (可选): 对明文 zip_data 签名
    let mut public_key_bytes: Vec<u8> = Vec::new();
    let mut signature_bytes: Vec<u8> = Vec::new();
    let mut has_signature = false;

    if let Some(pk) = private_key.filter(|k| !k.is_empty()) {
        let sk_clean = pk.trim();
        let sk_raw = match B64.decode(sk_clean) {
            Ok(b) => b,
            Err(e) => { eprintln!("错误: 私钥 Base64 解码失败: {}", e); std::process::exit(1); }
        };
        let secret_key = match SecretKey::from_slice(&sk_raw) {
            Ok(s) => s,
            Err(e) => { eprintln!("错误: 私钥格式无效: {:?}", e); std::process::exit(1); }
        };
        let key_pair = KeyPair::from_seed(secret_key.seed());
        public_key_bytes = key_pair.pk.to_vec();
        let signature = key_pair.sk.sign(&zip_data, None);
        signature_bytes = signature.to_vec();
        has_signature = true;
        println!("签名: 已用 Ed25519 私钥签名, 公钥指纹 {}", fingerprint(&public_key_bytes));
    }

    // 5. AES-256-GCM 加密
    let key = Key::<Aes256Gcm>::from_slice(&derived_key);
    let cipher = Aes256Gcm::new(key);
    let mut nonce_bytes = [0u8; 12];
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = match cipher.encrypt(nonce, zip_data.as_ref()) {
        Ok(c) => c,
        Err(e) => { eprintln!("错误: 加密失败: {}", e); std::process::exit(1); }
    };

    // 6. 写入 .axex 文件 (版本: 有签名=2, 无签名=1)
    let mut out = match fs::File::create(output_path) {
        Ok(f) => f,
        Err(e) => { eprintln!("错误: 创建文件失败: {}", e); std::process::exit(1); }
    };
    if let Err(e) = out.write_all(b"AXEX") { eprintln!("错误: 写入魔数失败: {}", e); std::process::exit(1); }
    let version: u8 = if has_signature { 2 } else { 1 };
    if let Err(e) = out.write_all(&[version]) { eprintln!("错误: 写入版本失败: {}", e); std::process::exit(1); }
    let plugin_id_bytes = plugin_id.as_bytes();
    if let Err(e) = out.write_all(&[plugin_id_bytes.len() as u8]) { eprintln!("错误: 写入插件ID长度失败: {}", e); std::process::exit(1); }
    if let Err(e) = out.write_all(plugin_id_bytes) { eprintln!("错误: 写入插件ID失败: {}", e); std::process::exit(1); }

    if has_signature {
        let pk_len = public_key_bytes.len() as u16;
        if let Err(e) = out.write_all(&pk_len.to_le_bytes()) { eprintln!("错误: 写入公钥长度失败: {}", e); std::process::exit(1); }
        if let Err(e) = out.write_all(&public_key_bytes) { eprintln!("错误: 写入公钥失败: {}", e); std::process::exit(1); }
        let sig_len = signature_bytes.len() as u16;
        if let Err(e) = out.write_all(&sig_len.to_le_bytes()) { eprintln!("错误: 写入签名长度失败: {}", e); std::process::exit(1); }
        if let Err(e) = out.write_all(&signature_bytes) { eprintln!("错误: 写入签名失败: {}", e); std::process::exit(1); }
    }

    if let Err(e) = out.write_all(&nonce_bytes) { eprintln!("错误: 写入 nonce 失败: {}", e); std::process::exit(1); }
    if let Err(e) = out.write_all(&ciphertext) { eprintln!("错误: 写入密文失败: {}", e); std::process::exit(1); }

    println!("成功: 已生成加密插件 {} ({} 个文件, AES-256-GCM", output_path, file_count);
    if has_signature {
        println!("      插件 ID: {}", plugin_id);
        println!("      签名: Ed25519, 公钥指纹 {}", fingerprint(&public_key_bytes));
    } else {
        println!("      插件 ID: {}", plugin_id);
        println!("      警告: 未签名 (v1)。如需作者身份标识, 请用 --private-key 签名生成 v2 包");
    }
    if is_default_key {
        println!("      警告: 使用默认加密密钥打包! 默认密钥仅用于开发阶段, 请勿用于分发。");
    }
}

// ============================================================================
// 生成插件模板 (.ns + index.html)
// ============================================================================
fn cmd_gen_template(dest_dir: &str, plugin_id: &str, plugin_name: &str, author: &str) {
    let dir = Path::new(dest_dir);
    if let Err(e) = fs::create_dir_all(dir) {
        eprintln!("错误: 创建目录失败: {}", e);
        std::process::exit(1);
    }

    // 生成 .ns 元数据文件 (NexusScript, 空格分隔语法)
    let ns_content = format!(
        "id \"{}\"\nname \"{}\"\nversion \"1.0.0\"\nauthor \"{}\"\ndescription \"插件功能描述\"\ntype \"local\"\nentry \"index.html\"\nlicense {{\n  type \"MIT\"\n  github \"\"\n  updateCheck \"\"\n}}\npermissions {{\n  storage {{ quota \"10MB\" }}\n  network {{ domains [] rateLimit 10 }}\n  clipboard {{}}\n  vibration {{}}\n  tts {{}}\n  media {{}}\n  notify {{}}\n  speech {{}}\n  file {{}}\n  ai {{\n    callable false\n    functions []\n  }}\n}}\n",
        plugin_id, plugin_name, author
    );
    let ns_path = dir.join(format!("{}.ns", plugin_id));
    if let Err(e) = fs::write(&ns_path, ns_content) {
        eprintln!("错误: 写入 .ns 失败: {}", e);
        std::process::exit(1);
    }
    println!("  已创建: {}", ns_path.display());

    // 生成 index.html 模板
    let html_template = r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0, viewport-fit=cover">
    <title>插件名称</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body { font-family: -apple-system, sans-serif; padding: env(safe-area-inset-top) env(safe-area-inset-right) env(safe-area-inset-bottom) env(safe-area-inset-left); }
    </style>
</head>
<body>
    <h1>插件内容</h1>
    <p>在此编写插件界面</p>
    <script>
        // 与宿主通信示例
        window.parent.postMessage({ source: 'axplugin', id: 'test', action: 'app.info', args: {} }, '*');
        window.addEventListener('message', function(e) {
            if (e.data.source !== 'axhost') return;
            console.log('收到宿主响应:', e.data);
        });
    </script>
</body>
</html>"#;
    let html_path = dir.join("index.html");
    if let Err(e) = fs::write(&html_path, html_template) {
        eprintln!("错误: 写入 index.html 失败: {}", e);
        std::process::exit(1);
    }
    println!("  已创建: {}", html_path.display());

    println!("成功: 插件模板已生成到 {}", dest_dir);
    println!("下一步: 编辑 index.html 实现插件逻辑, 然后运行:");
    println!("  axbuild pack {} my-plugin.axext", dest_dir);
    println!("  axbuild pack {} my-plugin.axex --encrypt", dest_dir);
}

// ============================================================================
// 生成 Ed25519 密钥对
// ============================================================================
fn cmd_keypair() {
    let key_pair = KeyPair::generate();
    let sk_b64 = B64.encode(key_pair.sk.to_vec());
    let pk_b64 = B64.encode(key_pair.pk.to_vec());
    let fp = fingerprint(key_pair.pk.as_ref());

    println!("Ed25519 密钥对已生成:");
    println!("  私钥 (base64, 保密! 用于签名 .axex):");
    println!("    {}", sk_b64);
    println!("  公钥 (base64, 可公开, 用于验签):");
    println!("    {}", pk_b64);
    println!("  公钥指纹: {}", fp);
    println!();
    println!("使用方式: axbuild pack ./my-plugin my-plugin.axex --encrypt --private-key \"{}\"", sk_b64);
    println!("注意: 私钥等同于你的作者身份, 切勿泄露或提交到版本库。");
}

// ============================================================================
// 查看插件包信息
// ============================================================================
fn cmd_info(path: &str, master_key: &str) {
    let file_path = Path::new(path);
    if !file_path.exists() {
        eprintln!("错误: 文件不存在: {}", path);
        std::process::exit(1);
    }

    let ext = file_path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "axext" => info_axext(path),
        "axex" => info_axex(path, master_key),
        _ => {
            eprintln!("错误: 不支持的插件格式, 需要 .axext 或 .axex");
            std::process::exit(1);
        }
    }
}

fn info_axext(path: &str) {
    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(e) => { eprintln!("错误: 打开文件失败: {}", e); std::process::exit(1); }
    };
    let mut zip = match zip::ZipArchive::new(file) {
        Ok(z) => z,
        Err(e) => { eprintln!("错误: 不是有效的 .axext 文件: {}", e); std::process::exit(1); }
    };

    let file_size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    println!("文件: {}", path);
    println!("大小: {} 字节 ({:.2} KB)", file_size, file_size as f64 / 1024.0);
    println!("格式: .axext (开源 ZIP)");
    println!("文件数: {}", zip.len());

    // 尝试读取任意一个 .ns 文件
    for i in 0..zip.len() {
        let name = zip.by_index(i).unwrap().name().to_string();
        if name.ends_with(".ns") {
            if let Ok(mut mf) = zip.by_name(&name) {
                let mut content = String::new();
                if mf.read_to_string(&mut content).is_ok() {
                    println!("\n{} 内容:", name);
                    // 简易打印顶层字段
                    for (k, v) in ns_root_scalars(&content) {
                        println!("  {}: {}", k, v);
                    }
                }
            }
            break;
        }
    }
}

fn info_axex(path: &str, master_key: &str) {
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(e) => { eprintln!("错误: 读取文件失败: {}", e); std::process::exit(1); }
    };

    if data.len() < 5 || &data[0..4] != b"AXEX" {
        eprintln!("错误: 不是有效的 .axex 文件 (魔数不匹配)");
        std::process::exit(1);
    }
    let version = data[4];
    if version != 1 && version != 2 {
        eprintln!("警告: 未知版本号: {}", version);
    }

    let plugin_id_len = data[5] as usize;
    if 6 + plugin_id_len >= data.len() {
        eprintln!("错误: 文件格式错误");
        std::process::exit(1);
    }
    let plugin_id = match std::str::from_utf8(&data[6..6 + plugin_id_len]) {
        Ok(s) => s,
        Err(_) => { eprintln!("错误: 插件ID编码错误"); std::process::exit(1); }
    };

    let mut offset = 6 + plugin_id_len;
    let mut has_signature = false;
    let mut public_key_b64 = String::new();
    let mut fp = String::new();
    let mut signature_valid: Option<bool> = None;

    if version >= 2 {
        if offset + 2 > data.len() { eprintln!("错误: 缺少公钥长度"); std::process::exit(1); }
        let pk_len = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
        offset += 2;
        if offset + pk_len > data.len() { eprintln!("错误: 公钥数据不足"); std::process::exit(1); }
        let public_key_raw = &data[offset..offset + pk_len];
        public_key_b64 = B64.encode(public_key_raw);
        fp = fingerprint(public_key_raw);
        offset += pk_len;
        has_signature = true;

        if offset + 2 > data.len() { eprintln!("错误: 缺少签名长度"); std::process::exit(1); }
        let sig_len = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
        offset += 2;
        if offset + sig_len > data.len() { eprintln!("错误: 签名数据不足"); std::process::exit(1); }
        let signature_raw = &data[offset..offset + sig_len];
        offset += sig_len;

        // 若提供密钥, 解密并验证签名
        if master_key.len() >= 8 {
            if offset + 12 > data.len() { eprintln!("错误: 缺少 nonce"); std::process::exit(1); }
            let nonce_bytes = &data[offset..offset + 12];
            let ciphertext = &data[offset + 12..];
            let key_material = if master_key.is_empty() { DEFAULT_MASTER_KEY } else { master_key };
            let mut derived_key = [0u8; 32];
            pbkdf2_hmac::<Sha256>(key_material.as_bytes(), plugin_id.as_bytes(), 100_000, &mut derived_key);
            let aes_key = Key::<Aes256Gcm>::from_slice(&derived_key);
            let cipher = Aes256Gcm::new(aes_key);
            let nonce = Nonce::from_slice(nonce_bytes);
            match cipher.decrypt(nonce, ciphertext) {
                Ok(zip_data) => {
                    match PublicKey::from_slice(public_key_raw) {
                        Ok(pk) => {
                            if let Ok(sig) = Signature::from_slice(signature_raw) {
                                signature_valid = Some(pk.verify(&zip_data, &sig).is_ok());
                            } else {
                                signature_valid = Some(false);
                            }
                        }
                        Err(_) => { signature_valid = Some(false); }
                    }
                }
                Err(_) => { signature_valid = None; }
            }
        }
    }

    let file_size = data.len();
    println!("文件: {}", path);
    println!("大小: {} 字节 ({:.2} KB)", file_size, file_size as f64 / 1024.0);
    println!("格式: .axex (AES-256-GCM 加密)");
    println!("版本: {}", version);
    println!("插件 ID: {}", plugin_id);
    if has_signature {
        println!("签名: Ed25519 (v2)");
        println!("公钥: {}", public_key_b64);
        println!("公钥指纹: {}", fp);
        match signature_valid {
            Some(true) => println!("签名验证: 通过 (密钥匹配且签名有效)"),
            Some(false) => println!("签名验证: 失败 (签名无效)"),
            None => println!("签名验证: 未验证 (未提供正确密钥或解密失败)"),
        }
    } else {
        println!("签名: 无 (v1, 不含作者身份)");
    }
    if master_key.is_empty() {
        println!("\n提示: 解密/验签需提供与打包时一致的密钥: axbuild info --master-key \"<key>\"");
    }
}
