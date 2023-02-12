CREATE TABLE subscription_token(
  subscription_token TEXT NOT NULL PRIMARY KEY,
  subscription_id UUID NOT NULL REFERENCES subscription(id)
);