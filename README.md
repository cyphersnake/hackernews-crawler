# Tech task
You should implement a web crawler for Hacker News. Crawl the first 10 pages of Hacker News and store enough information to be able to respond to the following through some APIs:
1.    List of top posts.
2.    List of posts sent by a user.
3.    List of posts of a user that was on the first page at some point.
 
DO NOT spend too much time on this question (max 4-5 hours). This question is open ended. What we are curious to see is your use of:
1.    Database use
2.    API design
3.    Concurrency
4.    Testing
5.    Containerisation and ease of deployment
 
It is not necessary to have a finished project, but it should be at a state where we can reliably get signals for above and to know that you are experienced in topics that will be needed during the job.
 
For example, there's no need to have significant coverage, but we expect to see sort of mocking during some test.
 
Requirements:
1. Use Rust.
2. Use a relational database.
 
# Solution
## Cralwer
For scrapping - I modified the standard example from [voyager](https://github.com/mattsse/voyager), it works quite slowly, but it works (with one little [issue](https://github.com/mattsse/voyager/issues/15))!

## API
For an external API, I took [gRPC](https://grpc.io/docs/what-is-grpc/introduction/) based on [protobuf](https://developers.google.com/protocol-buffers) and using [tonic](https://github.com/hyperium/tonic) crate for that. I love formats with a strict API specification to make writing clients as easy as possible.

## Database
Since part of the task was a relational database, and I also needed to quickly make a service, I took a lightweight [SQLite](https://www.sqlite.org/index.html) solution. 

For small projects, I prefer to use [sqlx](https://docs.rs/sqlx/latest/sqlx/) rather than a full ORM, because if I have 3-4 queries in system, I can use it to do it faster and more optimally, as well as have an arbitrary database structure (which is very convenient when prototyping), regardless of the limitations of any ready-to-use framework. This creates one inconvenience - the need to set env variable `DATABSE_URL` before building the project, because SQL syntax is checked for correctness by the compiler.

## Tests
The service itself works, however, I did not have time to write a normal client and did not complete full-fledged tests. Somewhere I left TODO, however, I hope I managed to demonstrate the approach to testing. I am a fan of TDD practice, however, I have to admit that it slows down development and it is difficult to fit such an approach into 6 hours of work. 
Moreover I love the [mockall](https://docs.rs/mockall/latest/mockall/) crate, however, on projects of this size, it doesn't provide much simplification.

## Not done
- Full unit tests
- Integration tests
- Dockerfile
- Divided into three crates so that when build the client, a `DATABASE_URL` is not needed

# How to Build

- [Install Rust](https://rustup.rs/)
- [Install sqlx](https://github.com/launchbadge/sqlx#install)

## Build
```bash
export DATABASE_URL="sqlite:posts.db?mode=rwc" # Choose database file
sqlx database reset # Create & Migrate Database
cargo build
```

## Run
## Server

### Default params
```bash
export DATABASE_URL="sqlite:posts.db?mode=rwc" # Choose database file
sqlx database reset # Create & Migrate Database
cargo run --bin server # For run with defaul params
```

### Custom params
```bash
export DATABASE_URL="sqlite:posts.db?mode=rwc"  # Choose database file
sqlx database reset # Create & Migrate Database
cp .env.example .env
vim .env
cargo run --bin server
```

## Client
```bash
export DATABASE_URL="sqlite:posts.db?mode=rwc"  # Choose database file
sqlx database reset # Create & Migrate Database
cargo run --bin client -- --help
```
