use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

/// Obtuve este método eficiente para leer líneas de https://doc.rust-lang.org/rust-by-example/std_misc/file/read_lines.html
/// The output is wrapped in a Result to allow matching on errors
/// Returns an Iterator to the Reader of the lines of the file.
pub fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    match File::open(filename) {
        Ok(file) => Ok(io::BufReader::new(file).lines()),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assert_read_lines() {
        let path = "src/tests/two_equal_orders.txt";
        match read_lines(path) {
            Ok(lines) => {
                let mut lines_count = 0;
                for line in lines {
                    match line {
                        Ok(line) => {
                            assert_eq!(line, "1,1,1,1");
                            lines_count += 1;
                        }
                        Err(e) => {
                            println!("[ERROR] Testeando read_lines: {:?}", e);
                        }
                    }
                }
                assert_eq!(lines_count, 2);
            }
            Err(e) => {
                println!("[ERROR] Testeando read_lines: {:?}", e);
            }
        }
    }
}
