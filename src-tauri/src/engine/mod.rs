pub mod loader;
pub mod scraper;
pub mod html_parser;
pub mod json_parser;
pub mod transformer;
pub mod extractor;
pub mod tags;
pub mod link_resolver;
pub mod host_detector;
pub mod download_manager;
pub mod webview_downloader;
pub mod navigator;
pub mod download_tracker;
pub mod library_tracker;
pub mod archive_extractor;
pub mod executable_detector;

pub use loader::SourceLoader;
pub use scraper::UniversalScraper;
#[allow(unused_imports)]
pub use navigator::Navigator;
#[allow(unused_imports)]
pub use download_tracker::{DownloadTracker, DownloadEntry, DownloadStatus, DownloadSignal, get_download_folder, generate_download_id};
#[allow(unused_imports)]
pub use download_manager::{streaming_download, StreamingDownloadError, DownloadPausedInfo};
#[allow(unused_imports)]
pub use library_tracker::{LibraryTracker, LibraryGame, LibraryGameStatus, GameExecutable, ExeType, get_library_folder};
#[allow(unused_imports)]
pub use archive_extractor::{ArchiveExtractor, ExtractionProgress, ExtractionResult};
#[allow(unused_imports)]
pub use executable_detector::ExecutableDetector;