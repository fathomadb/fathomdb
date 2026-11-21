use rusqlite::{blob::Blob, Connection};

enum Source<'a> {
    Null,
    Stored(Blob<'a>),
    Scalar(Vec<u8>),
}
pub(crate) struct Column<'a> {
    source: Source<'a>,
}
impl<'a> Column<'a> {
    pub(crate) fn open(
        c: &'a Connection,
        table: &str,
        column: &str,
        rowid: i64,
    ) -> Result<Self, &'static str> {
        let kind: String = c
            .query_row(
                &format!("SELECT typeof({column}) FROM {table} WHERE rowid=?1"),
                [rowid],
                |r| r.get(0),
            )
            .map_err(|_| "unavailable")?;
        let source = match kind.as_str() {
            "null" => Source::Null,
            "text" | "blob" => Source::Stored(
                c.blob_open("main", table, column, rowid, true).map_err(|_| "unavailable")?,
            ),
            "integer" => {
                let value: i64 = c
                    .query_row(
                        &format!("SELECT {column} FROM {table} WHERE rowid=?1"),
                        [rowid],
                        |r| r.get(0),
                    )
                    .map_err(|_| "unavailable")?;
                Source::Scalar(value.to_string().into_bytes())
            }
            "real" => {
                let value: f64 = c
                    .query_row(
                        &format!("SELECT {column} FROM {table} WHERE rowid=?1"),
                        [rowid],
                        |r| r.get(0),
                    )
                    .map_err(|_| "unavailable")?;
                if !value.is_finite() {
                    return Err("unsupported_value_encoding");
                }
                Source::Scalar(value.to_string().into_bytes())
            }
            _ => return Err("unsupported_value_encoding"),
        };
        Ok(Self { source })
    }
    pub(crate) fn len(&self) -> Option<usize> {
        match &self.source {
            Source::Null => None,
            Source::Stored(blob) => Some(blob.len()),
            Source::Scalar(bytes) => Some(bytes.len()),
        }
    }
    pub(crate) fn read(
        &self,
        offset: usize,
        limit: usize,
    ) -> Result<Option<Vec<u8>>, &'static str> {
        if limit > crate::storage::CHUNK {
            return Err("invalid_range");
        }
        let Some(total) = self.len() else {
            return if offset == 0 { Ok(None) } else { Err("invalid_range") };
        };
        if offset > total {
            return Err("invalid_range");
        }
        let n = limit.min(total - offset);
        match &self.source {
            Source::Stored(blob) => {
                let mut bytes = vec![0; n];
                blob.read_at_exact(&mut bytes, offset).map_err(|_| "unavailable")?;
                Ok(Some(bytes))
            }
            Source::Scalar(bytes) => Ok(Some(bytes[offset..offset + n].to_vec())),
            Source::Null => unreachable!(),
        }
    }
}
