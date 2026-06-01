# Database Migration Workflow

This project uses [sqlx](https://docs.rs/sqlx/latest/sqlx/migrate/index.html) for database migrations.

## Migration Version Control
- Migration files are stored in `backend/migrations/`.
- Each migration is a numbered `.sql` file (e.g., `001_create_anchors.sql`).
- Migration history is tracked in the database by sqlx.

## Creating a Migration
1. Create a new SQL file in `backend/migrations/` with the next sequential number.
2. Write your `-- up` (apply) and `-- down` (rollback) SQL statements.
3. Example:
   ```sql
   -- Add new table
   CREATE TABLE example (
       id INTEGER PRIMARY KEY,
       name TEXT NOT NULL
   );
   -- Down: drop table
   DROP TABLE example;
   ```

## Applying Migrations
- Migrations run automatically at backend startup.
- To run manually:
  ```sh
  cd backend
  cargo sqlx migrate run
  ```

## Rolling Back (Reverting) Migrations
- To revert the last migration:
  ```sh
  cd backend
  cargo sqlx migrate revert
  ```
- To revert multiple steps, repeat the command.

## CI/CD Integration
- Ensure migrations are run before app startup in deployment scripts.
- Example (Dockerfile or CI):
  ```sh
  cargo sqlx migrate run
  ```

## Best Practices
- Always test migrations locally before pushing.
- Use descriptive migration file names.
- Keep `-- down` (rollback) statements safe and idempotent.

## References
- [sqlx migration docs](https://docs.rs/sqlx/latest/sqlx/migrate/index.html)
