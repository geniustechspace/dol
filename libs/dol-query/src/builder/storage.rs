//! Storage builders — PUT, GET, LIST objects; READ, WRITE, MOVE files.
//!
//! These builders produce storage IR which has no SQL equivalent.
//! They will be rendered by future StorageBackend implementations.

use dol_core::ir::storage::*;

/// Builder for `PUT OBJECT` operations.
#[derive(Debug, Clone)]
pub struct PutObjectBuilder<'a> {
    key: String,
    bucket: String,
    source: ObjectSource<'a>,
    content_type: Option<String>,
    metadata: Vec<(String, String)>,
}

impl<'a> PutObjectBuilder<'a> {
    pub fn new(key: &str) -> Self {
        Self {
            key: key.to_string(),
            bucket: String::new(),
            source: ObjectSource::FromBytes,
            content_type: None,
            metadata: Vec::new(),
        }
    }

    pub fn into_bucket(mut self, bucket: &str) -> Self {
        self.bucket = bucket.to_string();
        self
    }

    pub fn from_path(mut self, path: &str) -> Self {
        self.source = ObjectSource::FromPath(path.to_string());
        self
    }

    pub fn from_bytes(mut self) -> Self {
        self.source = ObjectSource::FromBytes;
        self
    }

    pub fn content_type(mut self, ct: &str) -> Self {
        self.content_type = Some(ct.to_string());
        self
    }

    pub fn metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.push((key.to_string(), value.to_string()));
        self
    }

    pub fn build(self) -> PutObjectIR<'a> {
        PutObjectIR {
            key: self.key,
            bucket: self.bucket,
            source: self.source,
            content_type: self.content_type,
            metadata: self.metadata,
        }
    }
}

/// Builder for `GET OBJECT` operations.
#[derive(Debug, Clone)]
pub struct GetObjectBuilder {
    key: String,
    bucket: String,
}

impl GetObjectBuilder {
    pub fn new(key: &str) -> Self {
        Self {
            key: key.to_string(),
            bucket: String::new(),
        }
    }

    pub fn from_bucket(mut self, bucket: &str) -> Self {
        self.bucket = bucket.to_string();
        self
    }

    pub fn build(self) -> GetObjectIR {
        GetObjectIR {
            key: self.key,
            bucket: self.bucket,
        }
    }
}

/// Builder for `LIST OBJECTS` operations.
#[derive(Debug, Clone)]
pub struct ListObjectsBuilder {
    bucket: String,
    prefix: Option<String>,
    limit: Option<u64>,
    continuation_token: Option<String>,
}

impl ListObjectsBuilder {
    pub fn new() -> Self {
        Self {
            bucket: String::new(),
            prefix: None,
            limit: None,
            continuation_token: None,
        }
    }

    pub fn bucket(mut self, bucket: &str) -> Self {
        self.bucket = bucket.to_string();
        self
    }

    pub fn prefix(mut self, prefix: &str) -> Self {
        self.prefix = Some(prefix.to_string());
        self
    }

    pub fn limit(mut self, n: u64) -> Self {
        self.limit = Some(n);
        self
    }

    pub fn continuation_token(mut self, token: &str) -> Self {
        self.continuation_token = Some(token.to_string());
        self
    }

    pub fn build(self) -> ListObjectsIR {
        ListObjectsIR {
            bucket: self.bucket,
            prefix: self.prefix,
            limit: self.limit,
            continuation_token: self.continuation_token,
        }
    }
}

impl Default for ListObjectsBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for `READ FILE` operations.
#[derive(Debug, Clone)]
pub struct ReadFileBuilder {
    path: String,
    encoding: Option<String>,
}

impl ReadFileBuilder {
    pub fn new(path: &str) -> Self {
        Self {
            path: path.to_string(),
            encoding: None,
        }
    }

    pub fn encoding(mut self, enc: &str) -> Self {
        self.encoding = Some(enc.to_string());
        self
    }

    pub fn build(self) -> ReadFileIR {
        ReadFileIR {
            path: self.path,
            encoding: self.encoding,
        }
    }
}

/// Builder for `WRITE FILE` operations.
#[derive(Debug, Clone)]
pub struct WriteFileBuilder<'a> {
    path: String,
    source: ObjectSource<'a>,
    create_dirs: bool,
}

impl<'a> WriteFileBuilder<'a> {
    pub fn new(path: &str) -> Self {
        Self {
            path: path.to_string(),
            source: ObjectSource::FromBytes,
            create_dirs: false,
        }
    }

    pub fn from_bytes(mut self) -> Self {
        self.source = ObjectSource::FromBytes;
        self
    }

    pub fn from_path(mut self, source_path: &str) -> Self {
        self.source = ObjectSource::FromPath(source_path.to_string());
        self
    }

    pub fn create_dirs(mut self) -> Self {
        self.create_dirs = true;
        self
    }

    pub fn build(self) -> WriteFileIR<'a> {
        WriteFileIR {
            path: self.path,
            source: self.source,
            create_dirs: self.create_dirs,
        }
    }
}

/// Builder for `MOVE FILE` operations.
#[derive(Debug, Clone)]
pub struct MoveFileBuilder {
    from: String,
    to: String,
}

impl MoveFileBuilder {
    pub fn new(from: &str, to: &str) -> Self {
        Self {
            from: from.to_string(),
            to: to.to_string(),
        }
    }

    pub fn build(self) -> MoveFileIR {
        MoveFileIR {
            from: self.from,
            to: self.to,
        }
    }
}
