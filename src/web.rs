#![cfg(feature = "web")]

use {
    bytes::Buf,
    std::{
        env,
        error,
        fmt,
        fs,
        io,
        path::{
            Path,
            PathBuf,
        },
    },
    url::Url,
};

/// Get filename from URL
fn url_fname(url: &Url) -> Option<&str> {
    url
        .path_segments()
        .and_then(|segments| segments.last())
        .and_then(|name| if name.is_empty() { None } else { Some(name) })
}

/// A type specifying an error that occured during downloading a file
#[derive(Debug)]
pub enum DownloadError {
    /// Get request failed
    RequestError(reqwest::Error),
    /// Invalid URL
    ParseError(url::ParseError),
    /// Couldn't deduce filename from the URL
    NoFileNameError,
    /// Couldn't save the fetched file
    SaveError(io::Error),
}

impl From<url::ParseError> for DownloadError {
    #[inline]
    fn from(err: url::ParseError) -> DownloadError {
        DownloadError::ParseError(err)
    }
}

impl From<reqwest::Error> for DownloadError {
    #[inline]
    fn from(err: reqwest::Error) -> DownloadError {
        DownloadError::RequestError(err)
    }
}

impl From<io::Error> for DownloadError {
    #[inline]
    fn from(err: io::Error) -> DownloadError {
        DownloadError::SaveError(err)
    }
}

impl fmt::Display for DownloadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use DownloadError::*;
        match *self {
            NoFileNameError => write!(f, "couldn't infer file name from the url"),
            RequestError(ref e) => e.fmt(f),
            ParseError(ref e) => e.fmt(f),
            SaveError(ref e) => e.fmt(f),
        }
    }
}

impl error::Error for DownloadError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        use DownloadError::*;
        match *self {
            NoFileNameError => None,
            RequestError(ref e) => Some(e),
            ParseError(ref e) => Some(e),
            SaveError(ref e) => Some(e),
        }
    }
}

/// Download file if it doesn't already exist, and return the file's location.
/// 
/// You can use `out_dir` to specify the download directory, otherwise the build script output directory will be used.
/// 
/// Example:
/// 
/// ```
/// # fn run() -> Result<(), Box<dyn std::error::Error>> {
/// librarian::download_or_find_file("https://example.com/file.zip", None)?;
/// # Ok(())
/// # }
/// ```
pub fn download_or_find_file(
    url: &str,
    out_dir: Option<&Path>
) -> Result<PathBuf, DownloadError> {
    use DownloadError::*;

    let url_parsed = Url::parse(url)?;
    let fname = url_fname(&url_parsed);
    if let Some(fname) = fname {
        let out_dir = if let Some(out_dir) = out_dir {
            PathBuf::from(out_dir)
        } else {
            let out_dir = env::var("OUT_DIR").expect("You must provide the output directory when not running from a build script.");
            PathBuf::from(out_dir)
        };
        let path = out_dir.join(fname);
        if !path.exists() {
            let response = reqwest::blocking::get(url)?;
            let content = response.bytes()?;
            let mut bytes = content.bytes();
            let mut dest = fs::File::create(path.clone())?;
            io::copy(&mut bytes, &mut dest)?;
        }
        Ok(path)
    } else {
        Err(NoFileNameError)
    }
}

#[cfg(test)]
mod web_tests {
    #[test]
    fn download() {
        use crate::tests::dir_list_equals;
        use crate::*;
        let cur_file = Path::new(file!());
        let root = cur_file.parent().unwrap().parent().unwrap();
        let out = root.join("target").join("test").join("download");
        let _ = fs::remove_dir_all(out.as_path());
        let _ = fs::create_dir_all(out.as_path());
        let url = "https://httpbin.org/base64/YWJj";
        let out_expect = out.join("YWJj");
        assert_eq!(download_or_find_file(url, Some(out.as_path())).unwrap(), out_expect);
        assert_eq!(true, dir_list_equals(out.as_path(), vec![ "YWJj" ]));
        assert_eq!(
            fs::read_to_string(out_expect.as_path()).unwrap(),
            "abc".to_string()
        );
        let url = "http://invalid.url/but/the/file/is/still/cached/YWJj";
        assert_eq!(download_or_find_file(url, Some(out.as_path())).unwrap(), out_expect);
    }
}