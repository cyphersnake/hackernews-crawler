CREATE TABLE "posts"
(
    "post_id"                INT PRIMARY KEY,
    "title"                  VARCHAR   NOT NULL,
    "author"                 VARCHAR   NOT NULL,
    "url"                    VARCHAR   NOT NULL,
    "link"                   VARCHAR,
    "publication_moment"     TIMESTAMP NOT NULL,
    "last_snapshot_moment"   TIMESTAMP
);

CREATE TABLE "first_page_posts"
(
    "post_id"             INT,
    "snapshot_moment" TIMESTAMP NOT NULL,
    UNIQUE ("post_id", "snapshot_moment"),
    FOREIGN KEY ("post_id") REFERENCES "posts" ("post_id")
);

CREATE VIEW "posts_view" AS
SELECT "posts".*, "fpp"."snapshot_moment" IS NOT NULL AS "was_at_first_page"
FROM "posts"
         LEFT JOIN "first_page_posts" AS "fpp" ON "posts"."post_id" = "fpp"."post_id";

CREATE TRIGGER "posts_view"
    INSTEAD OF INSERT
    ON "posts_view"
BEGIN
    INSERT INTO "posts" ("post_id", "title", "author", "url", "link", "publication_moment", "last_snapshot_moment")
    VALUES ("new"."post_id", "new"."title", "new"."author", "new"."url", "new"."link", "new"."publication_moment", "new"."last_snapshot_moment")
    ON CONFLICT DO UPDATE SET "last_snapshot_moment" = "new"."last_snapshot_moment";

    INSERT INTO "first_page_posts" ("post_id", "snapshot_moment")
    SELECT "new"."post_id", "new"."last_snapshot_moment"
    WHERE "new"."was_at_first_page" IS TRUE;
END;
