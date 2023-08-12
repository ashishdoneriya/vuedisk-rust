const TEXT_EXTENSIONS: [&str; 113] = ["gnumakefile", "makefile", "ada", "adb", "ads", "ahk", "alg", "as", "ascx", "ashx", "asp", "aspx", "awk",
	"bash", "bat", "c", "cbl", "cc", "cfg", "cfm", "cfml", "clj", "cmf", "cob", "coffee", "config", "cpp",
	"cpy", "cs", "css", "csv", "cxx", "d", "dart", "e", "erl", "ex", "exs", "f", "f90", "f95", "fsx", "go",
	"groovy", "h", "hpp", "hrl", "hs", "htaccess", "htm", "html", "inc", "j", "jade", "java", "jl", "js",
	"json", "jsp", "kt", "liquid", "lisp", "log", "lsp", "lua", "m", "makefile", "md", "ml", "mli", "mm", "nim",
	"pas", "php", "pl", "pp", "prg", "pro", "properties", "ps1", "psm1", "pwn", "py", "r", "rb", "rkt", "rs",
	"rss", "sas", "sass", "scala", "scm", "scss", "sh", "sql", "st", "swift", "tcl", "text", "toml", "ts", "v",
	"vb", "vh", "vhd", "vhdl", "vm", "vue", "xml", "xsl", "xstl", "yaml", "zsh"];

pub fn is_text(file_extension: &String) -> bool {
	for extension in TEXT_EXTENSIONS {
		if file_extension.eq(extension) {
			return true;
		}
	}
	return false;
}
