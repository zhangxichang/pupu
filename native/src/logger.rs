pub fn init() {
    flexi_logger::Logger::with(flexi_logger::LogSpecification::info())
        .log_to_file(
            flexi_logger::FileSpec::default()
                .directory("logs")
                .suppress_basename(),
        )
        .format(flexi_logger::json_format)
        .rotate(
            flexi_logger::Criterion::Size(1024 * 1024),
            flexi_logger::Naming::Timestamps,
            flexi_logger::Cleanup::KeepCompressedFiles(10),
        )
        .start()
        .unwrap();
    log::info!("日志开始记录");
}
