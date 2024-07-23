use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

use crate::AppError;

use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};

/// 聊天文件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatFile {
    /// Workspace ID
    pub ws_id: u64,
    /// 扩展名
    pub ext: String,
    /// Hash
    pub hash: String,
}

/// 实现ChatFile
impl ChatFile {
    /// 根据工作空间，文件名和数据创建ChatFile 数据实际上是为了计算Hash
    pub fn new(ws_id: u64, filename: &str, data: &[u8]) -> Self {
        let hash = Sha1::digest(data);
        Self {
            ws_id,
            ext: filename.split('.').last().unwrap_or("txt").to_string(),
            hash: hex::encode(hash),
        }
    }

    /// 获取URL
    pub fn url(&self) -> String {
        format!("/files/{}", self.hash_to_path())
    }
    /// 获取物理路径
    pub fn path(&self, base_dir: &Path) -> PathBuf {
        base_dir.join(self.hash_to_path())
    }

    /// 将Hash转换为物理路径
    pub fn hash_to_path(&self) -> String {
        let (part1, part2) = self.hash.split_at(3);
        let (part2, part3) = part2.split_at(3);

        format!("{}/{}/{}/{}.{}", self.ws_id, part1, part2, part3, self.ext)
    }
}

/// 从路径字符串解析ChatFile
impl FromStr for ChatFile {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // 尝试检查去掉前缀
        let Some(s) = s.strip_prefix("/files/") else {
            return Err(AppError::ChatFileError(format!(
                "invalid chat file path:{}",
                s
            )));
        };

        // 分割路径
        let parts: Vec<&str> = s.split('/').collect();

        // 判断是否为4段
        if parts.len() != 4 {
            return Err(AppError::ChatFileError(format!(
                "invalid chat file path:{}",
                s
            )));
        }
        // 从第一段获取workspace id
        let Ok(ws_id) = parts[0].parse::<u64>() else {
            return Err(AppError::ChatFileError("invalid ws_id".to_string()));
        };
        // 从最后一段获取文件最后一段Hash和扩展名
        let Some((part3, ext)) = parts[3].split_once('.') else {
            return Err(AppError::ChatFileError("invalid file name".to_string()));
        };
        // 生成Hash
        let hash = format!("{}{}{}", parts[1], parts[2], part3);

        Ok(Self {
            ws_id,
            ext: ext.to_string(),
            hash,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_file_name_should_work() {
        let file = ChatFile::new(1, "test.txt", b"hello");
        assert_eq!(file.ws_id, 1);
        assert_eq!(file.ext, "txt");
        assert_eq!(file.hash, "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d");
    }
}
