# Running Migrator CLI

- Generate a new migration file
    ```sh
    sea-orm-cli/cargo run -- generate MIGRATION_NAME
    ```
- Apply all pending migrations
    ```sh
    sea-orm-cli/cargo run
    ```
    ```sh
    sea-orm-cli/cargo run -- up
    ```
- Apply first 10 pending migrations
    ```sh
    sea-orm-cli/cargo run -- up -n 10
    ```
- Rollback last applied migrations
    ```sh
    sea-orm-cli/cargo run -- down
    ```
- Rollback last 10 applied migrations
    ```sh
    sea-orm-cli/cargo run -- down -n 10
    ```
- Drop all tables from the database, then reapply all migrations
    ```sh
    sea-orm-cli/cargo run -- fresh
    ```
- Rollback all applied migrations, then reapply all migrations
    ```sh
    sea-orm-cli/cargo run -- refresh
    ```
- Rollback all applied migrations
    ```sh
    sea-orm-cli/cargo run -- reset
    ```
- Check the status of all migrations
    ```sh
    sea-orm-cli/cargo run -- status
    ```
