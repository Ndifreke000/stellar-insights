#[cfg(test)]
mod tests {
    use crate::models::{CreateSponsorshipRequest, Sponsorship};
    use crate::services::SponsorshipTrackerService;
    use sqlx::SqlitePool;

    #[tokio::test]
    async fn test_track_sponsorship() {
        let pool = create_test_db().await;
        let service = SponsorshipTrackerService::new(pool);

        let result = service
            .track_sponsorship(
                "GBUQWP3BOUZX34ULNQG23RQ6F4BFXWXZMVEON46YSVP33ZDTF75SZK2S".to_string(),
                "GCKFBEOZXN3IXMF64APXMC7FXZUQZU6ZGWKD2XXWQVM3CWDUSXRKQFB".to_string(),
                2,
                "100.00".to_string(),
            )
            .await;

        assert!(result.is_ok());
        let sponsorship = result.unwrap();
        assert_eq!(sponsorship.sponsor, "GBUQWP3BOUZX34ULNQG23RQ6F4BFXWXZMVEON46YSVP33ZDTF75SZK2S");
        assert_eq!(sponsorship.total_amount, "100.00");
    }

    #[tokio::test]
    async fn test_get_all_sponsorships() {
        let pool = create_test_db().await;
        let service = SponsorshipTrackerService::new(pool);

        service
            .track_sponsorship(
                "SPONSOR1".to_string(),
                "ACCOUNT1".to_string(),
                1,
                "50.00".to_string(),
            )
            .await
            .unwrap();

        service
            .track_sponsorship(
                "SPONSOR2".to_string(),
                "ACCOUNT2".to_string(),
                2,
                "75.00".to_string(),
            )
            .await
            .unwrap();

        let sponsorships = service.get_all_sponsorships(10, 0).await.unwrap();
        assert_eq!(sponsorships.len(), 2);
    }

    #[tokio::test]
    async fn test_get_sponsorships_by_sponsor() {
        let pool = create_test_db().await;
        let service = SponsorshipTrackerService::new(pool);

        let sponsor = "SPONSOR_TEST".to_string();

        service
            .track_sponsorship(
                sponsor.clone(),
                "ACCOUNT1".to_string(),
                1,
                "50.00".to_string(),
            )
            .await
            .unwrap();

        service
            .track_sponsorship(
                sponsor.clone(),
                "ACCOUNT2".to_string(),
                1,
                "75.00".to_string(),
            )
            .await
            .unwrap();

        let sponsorships = service
            .get_sponsorships_by_sponsor(sponsor)
            .await
            .unwrap();

        assert_eq!(sponsorships.len(), 2);
    }

    #[tokio::test]
    async fn test_update_sponsorship() {
        let pool = create_test_db().await;
        let service = SponsorshipTrackerService::new(pool);

        let sponsorship = service
            .track_sponsorship(
                "SPONSOR".to_string(),
                "ACCOUNT".to_string(),
                2,
                "100.00".to_string(),
            )
            .await
            .unwrap();

        let updated = service
            .update_sponsorship(sponsorship.id, 3, "150.00".to_string())
            .await
            .unwrap();

        assert_eq!(updated.total_amount, "150.00");
        assert_eq!(updated.sponsored_reserves, 3);
    }

    #[tokio::test]
    async fn test_get_sponsorship_history() {
        let pool = create_test_db().await;
        let service = SponsorshipTrackerService::new(pool);

        let sponsorship = service
            .track_sponsorship(
                "SPONSOR".to_string(),
                "ACCOUNT".to_string(),
                1,
                "100.00".to_string(),
            )
            .await
            .unwrap();

        service
            .update_sponsorship(sponsorship.id.clone(), 2, "150.00".to_string())
            .await
            .unwrap();

        let history = service
            .get_sponsorship_history(sponsorship.id)
            .await
            .unwrap();

        assert!(history.len() >= 1);
    }

    #[tokio::test]
    async fn test_get_analytics() {
        let pool = create_test_db().await;
        let service = SponsorshipTrackerService::new(pool);

        service
            .track_sponsorship(
                "SPONSOR1".to_string(),
                "ACCOUNT1".to_string(),
                1,
                "100.00".to_string(),
            )
            .await
            .unwrap();

        service
            .track_sponsorship(
                "SPONSOR2".to_string(),
                "ACCOUNT2".to_string(),
                2,
                "200.00".to_string(),
            )
            .await
            .unwrap();

        let analytics = service.get_analytics().await.unwrap();
        assert_eq!(analytics.total_sponsorships, 2);
        assert_eq!(analytics.unique_sponsors, 2);
        assert_eq!(analytics.unique_sponsored_accounts, 2);
    }

    async fn create_test_db() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        crate::db::init(&pool).await.unwrap();
        pool
    }
}
