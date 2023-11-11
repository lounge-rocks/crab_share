# 0.2.2

- Fix `--zip-single-file` option for files bigger than 2^32 bytes
- Update dependencies
- Security updates (<https://github.com/imsnif/bandwhich/issues/284#issuecomment-1754321993>)

# 0.2.0

- Use multithreading for multipart upload
- Add `--zip-single-file` and `--compression` options
- Add progress bars for operations
- Move to rusty-s3
- Move to ulid to support purging expired files

# 0.1.1

- Fix folder upload
