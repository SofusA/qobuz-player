CREATE TABLE IF NOT EXISTS "cache_entries" (
	"track_id" int primary key not null,
    "path" text not null,
    "last_opened" text not null
);
