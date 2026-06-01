# Database Migration System

This project uses `sqlx` for automated, version-controlled database migrations with rollback support.

## Migration Workflow

- **Location:** All migration files are in `backend/migrations/`.
- **Versioning:** Each migration is a sequentially numbered `.sql` file (e.g., `001_create_anchors.sql`).
- **Tracking:** Applied migrations are tracked in the database by `sqlx`.

## Running Migrations

- **Automatic:** Migrations run at backend startup.
- **Manual:**
  ```sh
  ./scripts/run_migrations.sh
  ```

## Rolling Back Migrations

- **Revert last migration:**
  ```sh
  ./scripts/rollback_last_migration.sh
  ```
- **Multiple rollbacks:** Run the script multiple times.

## CI/CD Integration

- The Docker build and entrypoint ensure migrations run before app startup.
- For custom CI/CD, call `./scripts/run_migrations.sh` before launching the backend.

## Best Practices

- Always provide both `up` and `down` SQL in migration files.
- Test migrations and rollbacks locally before pushing.
- Use descriptive migration names.

## References
- See `backend/MIGRATIONS.md` for full details.
- See `scripts/` for helper scripts.
