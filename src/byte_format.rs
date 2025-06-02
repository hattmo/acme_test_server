use std::fmt::Display;

pub struct ByteFormat<T>(pub T);

impl<T> Display for ByteFormat<T>
where
    T: AsRef<[u8]>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self(bytes) = self;
        let bytes = bytes.as_ref();
        let bytes_iter = bytes.iter().map(|&b| {
            if b.is_ascii_alphanumeric() {
                if f.alternate() {
                    format!("\x1b[31m{:>2.2}\x1b[m ", (b as char).to_string())
                } else {
                    format!("{:>2.2} ", (b as char).to_string())
                }
            } else {
                format!("{b:>02.2X} ")
            }
        });
        let bytes_vec: Vec<String> = if let Some(precision) = f.precision() {
            let mut bytes_vec: Vec<String> = bytes_iter.take(precision).collect();
            if bytes_vec.len() < bytes.len()
                && let Some(last) = bytes_vec.last_mut() {
                    *last = "...".to_string();
                }
            bytes_vec
        } else {
            bytes_iter.collect()
        };
        if let Some(width) = f.width() {
            for row in bytes_vec.chunks(width) {
                let row: String = row.iter().map(String::as_str).collect();
                writeln!(f, "{row}")?;
            }
        } else {
            let out: String = bytes_vec.iter().map(String::as_str).collect();
            write!(f, "{out}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::ByteFormat;

    #[test]
    fn test_bytes() {
        let b = [1, 2, 3, 4, 5, 6, 7, 8, 9, 65];
        let bytes = ByteFormat(&b);
        println!("{bytes:#}");
        assert!(false);
    }
}
