use regex::RegexBuilder;

pub fn parse_javac_output() {
    let regex = RegexBuilder::new(r"^(?P<file>.+):(?P<line>\d+): (warning|error): (?P<error>.+)$")
        .multi_line(true)
        .case_insensitive(true)
        .build()
        .unwrap();
    let string = "srcjava\\Main.java:4: error: ',', ')', or '[' expected
    public static void main(String[] args|) {
                                         ^
srcjava\\Main.java:5: error: ';' expected
        System.out.println(Bruh.getHello() + \" from java\")
                                                          ^
2 errors";

    // result will be an iterator over tuples containing the start and end indices for each match in the string
    let result = regex.captures_iter(string);

    for mat in result {
        println!(" match {:?}", mat);
    }
}
