# Lua Comment Stripper

This project will strip the comments out of any provided lua file replacing them with either empty lines
or whitespace. The input can either be a file or directory, the output should be a directory.

There is an option to calculate line diffs also, if a directory is provided for this, the same directory
tree will be provided but the files will be a `.diff` file instead of the original `.lua`.

## Usage

```shell
Usage: lua-comment-stripper [OPTIONS] <INPUT> <OUTPUT>

Arguments:
  <INPUT>   The input directory
  <OUTPUT>  The output directory

Options:
      --diff-dir <DIFF_DIR>  The directory to output diff files
  -d, --diff-verbose         If provided will output the full file diffs including whitespace and comments
  -c, --clean                Clean the output directory before writing
  -h, --help                 Print help
```

### `diff-dir`

This argument can be provided to audit the comment stripping. If everything was successful, the
expectation is that this directory would be empty/not be created. In the event that we find any
changes that don't map to purely whitespace or comments, a `.diff` file will be generated in this
directory on the same relative path as the original `.lua` file.
