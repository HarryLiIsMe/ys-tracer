use super::user::*;
use crate::state::AppStateRaw;

#[async_trait]
pub trait IUser: std::ops::Deref<Target = AppStateRaw> {
    async fn adress_add(&self, address: &str, experience: &str, post_email :&str) -> sqlx::Result<u64>
    {
        /*sqlx::query!(
            r#"
        INSERT INTO user_address2 (address, experience, post_email)
       VALUES ($1 ,$2, $3)
                "#,
            address,
            experience,
            post_email
        )
            .execute(&self.sql)
            .await
            .map(|d| d.rows_affected())*/

        let sql = format!(
            "  INSERT INTO user_address2 (address, experience, post_email)
       VALUES ('{}' ,'{}', '{}');",
            address,
            experience,
            post_email
        );
        sqlx::query(&sql).bind(address).execute(&self.sql).await.map(|d| d.rows_affected())
    }

   async fn adress_query(&self, address: &str) -> sqlx::Result<AddressExperience> {

        let sql = format!(
            "SELECT address, experience, post_email
            FROM user_address2
            where address = '{}';",
            address
        );
        sqlx::query_as::<_, AddressExperience>(&sql).bind(address).fetch_one(&self.sql).await
    }

    async fn email_query(&self, email: &str) -> sqlx::Result<AddressExperience> {

        let sql = format!(
            "SELECT address, experience, post_email
            FROM user_address2
            where post_email = '{}';",
            email
        );
        sqlx::query_as::<_, AddressExperience>(&sql).bind(email).fetch_one(&self.sql).await
    }

    async fn adress_all(&self, limit: i16, offset:i16) -> sqlx::Result<Vec<AddressExperience>> {

        let sql = format!(
            "SELECT address, experience
            FROM user_address2 order by experience desc
            limit {} offset {} ;",
            limit, offset
        );
        sqlx::query_as(&sql).bind(1).fetch_all(&self.sql).await
    }

    async fn adress_update(&self, address: &str, experience: &str) ->sqlx::Result<u64> {

        let sql = format!(
            "update user_address2 set experience='{}' where address='{}';",
            experience, address
        );

        sqlx::query(&sql).bind(address).execute(&self.sql).await.map(|d| d.rows_affected())
    }
}

#[cfg(any(feature = "postgres"))]
#[async_trait]
impl IUser for &AppStateRaw {

}

