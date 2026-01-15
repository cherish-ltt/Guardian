-- ----------------------------
-- Function structure for armor
-- ----------------------------
DROP FUNCTION IF EXISTS "public"."armor"(bytea);
CREATE FUNCTION "public"."armor"(bytea)
  RETURNS "pg_catalog"."text" AS '$libdir/pgcrypto', 'pg_armor'
  LANGUAGE c IMMUTABLE STRICT
  COST 1;
ALTER FUNCTION "public"."armor"(bytea) OWNER TO "postgres";

-- ----------------------------
-- Function structure for armor
-- ----------------------------
DROP FUNCTION IF EXISTS "public"."armor"(bytea, _text, _text);
CREATE FUNCTION "public"."armor"(bytea, _text, _text)
  RETURNS "pg_catalog"."text" AS '$libdir/pgcrypto', 'pg_armor'
  LANGUAGE c IMMUTABLE STRICT
  COST 1;
ALTER FUNCTION "public"."armor"(bytea, _text, _text) OWNER TO "postgres";

-- ----------------------------
-- Function structure for crypt
-- ----------------------------
DROP FUNCTION IF EXISTS "public"."crypt"(text, text);
CREATE FUNCTION "public"."crypt"(text, text)
  RETURNS "pg_catalog"."text" AS '$libdir/pgcrypto', 'pg_crypt'
  LANGUAGE c IMMUTABLE STRICT
  COST 1;
ALTER FUNCTION "public"."crypt"(text, text) OWNER TO "postgres";

-- ----------------------------
-- Function structure for dearmor
-- ----------------------------
DROP FUNCTION IF EXISTS "public"."dearmor"(text);
CREATE FUNCTION "public"."dearmor"(text)
  RETURNS "pg_catalog"."bytea" AS '$libdir/pgcrypto', 'pg_dearmor'
  LANGUAGE c IMMUTABLE STRICT
  COST 1;
ALTER FUNCTION "public"."dearmor"(text) OWNER TO "postgres";

-- ----------------------------
-- Function structure for decrypt
-- ----------------------------
DROP FUNCTION IF EXISTS "public"."decrypt"(bytea, bytea, text);
CREATE FUNCTION "public"."decrypt"(bytea, bytea, text)
  RETURNS "pg_catalog"."bytea" AS '$libdir/pgcrypto', 'pg_decrypt'
  LANGUAGE c IMMUTABLE STRICT
  COST 1;
ALTER FUNCTION "public"."decrypt"(bytea, bytea, text) OWNER TO "postgres";

-- ----------------------------
-- Function structure for decrypt_iv
-- ----------------------------
DROP FUNCTION IF EXISTS "public"."decrypt_iv"(bytea, bytea, bytea, text);
CREATE FUNCTION "public"."decrypt_iv"(bytea, bytea, bytea, text)
  RETURNS "pg_catalog"."bytea" AS '$libdir/pgcrypto', 'pg_decrypt_iv'
  LANGUAGE c IMMUTABLE STRICT
  COST 1;
ALTER FUNCTION "public"."decrypt_iv"(bytea, bytea, bytea, text) OWNER TO "postgres";

-- ----------------------------
-- Function structure for digest
-- ----------------------------
DROP FUNCTION IF EXISTS "public"."digest"(text, text);
CREATE FUNCTION "public"."digest"(text, text)
  RETURNS "pg_catalog"."bytea" AS '$libdir/pgcrypto', 'pg_digest'
  LANGUAGE c IMMUTABLE STRICT
  COST 1;
ALTER FUNCTION "public"."digest"(text, text) OWNER TO "postgres";

-- ----------------------------
-- Function structure for digest
-- ----------------------------
DROP FUNCTION IF EXISTS "public"."digest"(bytea, text);
CREATE FUNCTION "public"."digest"(bytea, text)
  RETURNS "pg_catalog"."bytea" AS '$libdir/pgcrypto', 'pg_digest'
  LANGUAGE c IMMUTABLE STRICT
  COST 1;
ALTER FUNCTION "public"."digest"(bytea, text) OWNER TO "postgres";

-- ----------------------------
-- Function structure for encrypt
-- ----------------------------
DROP FUNCTION IF EXISTS "public"."encrypt"(bytea, bytea, text);
CREATE FUNCTION "public"."encrypt"(bytea, bytea, text)
  RETURNS "pg_catalog"."bytea" AS '$libdir/pgcrypto', 'pg_encrypt'
  LANGUAGE c IMMUTABLE STRICT
  COST 1;
ALTER FUNCTION "public"."encrypt"(bytea, bytea, text) OWNER TO "postgres";

-- ----------------------------
-- Function structure for encrypt_iv
-- ----------------------------
DROP FUNCTION IF EXISTS "public"."encrypt_iv"(bytea, bytea, bytea, text);
CREATE FUNCTION "public"."encrypt_iv"(bytea, bytea, bytea, text)
  RETURNS "pg_catalog"."bytea" AS '$libdir/pgcrypto', 'pg_encrypt_iv'
  LANGUAGE c IMMUTABLE STRICT
  COST 1;
ALTER FUNCTION "public"."encrypt_iv"(bytea, bytea, bytea, text) OWNER TO "postgres";

-- ----------------------------
-- Function structure for gen_random_bytes
-- ----------------------------
DROP FUNCTION IF EXISTS "public"."gen_random_bytes"(int4);
CREATE FUNCTION "public"."gen_random_bytes"(int4)
  RETURNS "pg_catalog"."bytea" AS '$libdir/pgcrypto', 'pg_random_bytes'
  LANGUAGE c VOLATILE STRICT
  COST 1;
ALTER FUNCTION "public"."gen_random_bytes"(int4) OWNER TO "postgres";

-- ----------------------------
-- Function structure for gen_random_uuid
-- ----------------------------
DROP FUNCTION IF EXISTS "public"."gen_random_uuid"();
CREATE FUNCTION "public"."gen_random_uuid"()
  RETURNS "pg_catalog"."uuid" AS '$libdir/pgcrypto', 'pg_random_uuid'
  LANGUAGE c VOLATILE
  COST 1;
ALTER FUNCTION "public"."gen_random_uuid"() OWNER TO "postgres";

-- ----------------------------
-- Function structure for gen_salt
-- ----------------------------
DROP FUNCTION IF EXISTS "public"."gen_salt"(text);
CREATE FUNCTION "public"."gen_salt"(text)
  RETURNS "pg_catalog"."text" AS '$libdir/pgcrypto', 'pg_gen_salt'
  LANGUAGE c VOLATILE STRICT
  COST 1;
ALTER FUNCTION "public"."gen_salt"(text) OWNER TO "postgres";

-- ----------------------------
-- Function structure for gen_salt
-- ----------------------------
DROP FUNCTION IF EXISTS "public"."gen_salt"(text, int4);
CREATE FUNCTION "public"."gen_salt"(text, int4)
  RETURNS "pg_catalog"."text" AS '$libdir/pgcrypto', 'pg_gen_salt_rounds'
  LANGUAGE c VOLATILE STRICT
  COST 1;
ALTER FUNCTION "public"."gen_salt"(text, int4) OWNER TO "postgres";

-- ----------------------------
-- Function structure for hmac
-- ----------------------------
DROP FUNCTION IF EXISTS "public"."hmac"(bytea, bytea, text);
CREATE FUNCTION "public"."hmac"(bytea, bytea, text)
  RETURNS "pg_catalog"."bytea" AS '$libdir/pgcrypto', 'pg_hmac'
  LANGUAGE c IMMUTABLE STRICT
  COST 1;
ALTER FUNCTION "public"."hmac"(bytea, bytea, text) OWNER TO "postgres";

-- ----------------------------
-- Function structure for hmac
-- ----------------------------
DROP FUNCTION IF EXISTS "public"."hmac"(text, text, text);
CREATE FUNCTION "public"."hmac"(text, text, text)
  RETURNS "pg_catalog"."bytea" AS '$libdir/pgcrypto', 'pg_hmac'
  LANGUAGE c IMMUTABLE STRICT
  COST 1;
ALTER FUNCTION "public"."hmac"(text, text, text) OWNER TO "postgres";

-- ----------------------------
-- Function structure for pgp_armor_headers
-- ----------------------------
DROP FUNCTION IF EXISTS "public"."pgp_armor_headers"(text, OUT "key" text, OUT "value" text);
CREATE FUNCTION "public"."pgp_armor_headers"(IN text, OUT "key" text, OUT "value" text)
  RETURNS SETOF "pg_catalog"."record" AS '$libdir/pgcrypto', 'pgp_armor_headers'
  LANGUAGE c IMMUTABLE STRICT
  COST 1
  ROWS 1000;
ALTER FUNCTION "public"."pgp_armor_headers"(text, OUT "key" text, OUT "value" text) OWNER TO "postgres";

-- ----------------------------
-- Function structure for pgp_key_id
-- ----------------------------
DROP FUNCTION IF EXISTS "public"."pgp_key_id"(bytea);
CREATE FUNCTION "public"."pgp_key_id"(bytea)
  RETURNS "pg_catalog"."text" AS '$libdir/pgcrypto', 'pgp_key_id_w'
  LANGUAGE c IMMUTABLE STRICT
  COST 1;
ALTER FUNCTION "public"."pgp_key_id"(bytea) OWNER TO "postgres";

-- ----------------------------
-- Function structure for pgp_pub_decrypt
-- ----------------------------
DROP FUNCTION IF EXISTS "public"."pgp_pub_decrypt"(bytea, bytea);
CREATE FUNCTION "public"."pgp_pub_decrypt"(bytea, bytea)
  RETURNS "pg_catalog"."text" AS '$libdir/pgcrypto', 'pgp_pub_decrypt_text'
  LANGUAGE c IMMUTABLE STRICT
  COST 1;
ALTER FUNCTION "public"."pgp_pub_decrypt"(bytea, bytea) OWNER TO "postgres";

-- ----------------------------
-- Function structure for pgp_pub_decrypt
-- ----------------------------
DROP FUNCTION IF EXISTS "public"."pgp_pub_decrypt"(bytea, bytea, text, text);
CREATE FUNCTION "public"."pgp_pub_decrypt"(bytea, bytea, text, text)
  RETURNS "pg_catalog"."text" AS '$libdir/pgcrypto', 'pgp_pub_decrypt_text'
  LANGUAGE c IMMUTABLE STRICT
  COST 1;
ALTER FUNCTION "public"."pgp_pub_decrypt"(bytea, bytea, text, text) OWNER TO "postgres";

-- ----------------------------
-- Function structure for pgp_pub_decrypt
-- ----------------------------
DROP FUNCTION IF EXISTS "public"."pgp_pub_decrypt"(bytea, bytea, text);
CREATE FUNCTION "public"."pgp_pub_decrypt"(bytea, bytea, text)
  RETURNS "pg_catalog"."text" AS '$libdir/pgcrypto', 'pgp_pub_decrypt_text'
  LANGUAGE c IMMUTABLE STRICT
  COST 1;
ALTER FUNCTION "public"."pgp_pub_decrypt"(bytea, bytea, text) OWNER TO "postgres";

-- ----------------------------
-- Function structure for pgp_pub_decrypt_bytea
-- ----------------------------
DROP FUNCTION IF EXISTS "public"."pgp_pub_decrypt_bytea"(bytea, bytea, text, text);
CREATE FUNCTION "public"."pgp_pub_decrypt_bytea"(bytea, bytea, text, text)
  RETURNS "pg_catalog"."bytea" AS '$libdir/pgcrypto', 'pgp_pub_decrypt_bytea'
  LANGUAGE c IMMUTABLE STRICT
  COST 1;
ALTER FUNCTION "public"."pgp_pub_decrypt_bytea"(bytea, bytea, text, text) OWNER TO "postgres";

-- ----------------------------
-- Function structure for pgp_pub_decrypt_bytea
-- ----------------------------
DROP FUNCTION IF EXISTS "public"."pgp_pub_decrypt_bytea"(bytea, bytea);
CREATE FUNCTION "public"."pgp_pub_decrypt_bytea"(bytea, bytea)
  RETURNS "pg_catalog"."bytea" AS '$libdir/pgcrypto', 'pgp_pub_decrypt_bytea'
  LANGUAGE c IMMUTABLE STRICT
  COST 1;
ALTER FUNCTION "public"."pgp_pub_decrypt_bytea"(bytea, bytea) OWNER TO "postgres";

-- ----------------------------
-- Function structure for pgp_pub_decrypt_bytea
-- ----------------------------
DROP FUNCTION IF EXISTS "public"."pgp_pub_decrypt_bytea"(bytea, bytea, text);
CREATE FUNCTION "public"."pgp_pub_decrypt_bytea"(bytea, bytea, text)
  RETURNS "pg_catalog"."bytea" AS '$libdir/pgcrypto', 'pgp_pub_decrypt_bytea'
  LANGUAGE c IMMUTABLE STRICT
  COST 1;
ALTER FUNCTION "public"."pgp_pub_decrypt_bytea"(bytea, bytea, text) OWNER TO "postgres";

-- ----------------------------
-- Function structure for pgp_pub_encrypt
-- ----------------------------
DROP FUNCTION IF EXISTS "public"."pgp_pub_encrypt"(text, bytea, text);
CREATE FUNCTION "public"."pgp_pub_encrypt"(text, bytea, text)
  RETURNS "pg_catalog"."bytea" AS '$libdir/pgcrypto', 'pgp_pub_encrypt_text'
  LANGUAGE c VOLATILE STRICT
  COST 1;
ALTER FUNCTION "public"."pgp_pub_encrypt"(text, bytea, text) OWNER TO "postgres";

-- ----------------------------
-- Function structure for pgp_pub_encrypt
-- ----------------------------
DROP FUNCTION IF EXISTS "public"."pgp_pub_encrypt"(text, bytea);
CREATE FUNCTION "public"."pgp_pub_encrypt"(text, bytea)
  RETURNS "pg_catalog"."bytea" AS '$libdir/pgcrypto', 'pgp_pub_encrypt_text'
  LANGUAGE c VOLATILE STRICT
  COST 1;
ALTER FUNCTION "public"."pgp_pub_encrypt"(text, bytea) OWNER TO "postgres";

-- ----------------------------
-- Function structure for pgp_pub_encrypt_bytea
-- ----------------------------
DROP FUNCTION IF EXISTS "public"."pgp_pub_encrypt_bytea"(bytea, bytea);
CREATE FUNCTION "public"."pgp_pub_encrypt_bytea"(bytea, bytea)
  RETURNS "pg_catalog"."bytea" AS '$libdir/pgcrypto', 'pgp_pub_encrypt_bytea'
  LANGUAGE c VOLATILE STRICT
  COST 1;
ALTER FUNCTION "public"."pgp_pub_encrypt_bytea"(bytea, bytea) OWNER TO "postgres";

-- ----------------------------
-- Function structure for pgp_pub_encrypt_bytea
-- ----------------------------
DROP FUNCTION IF EXISTS "public"."pgp_pub_encrypt_bytea"(bytea, bytea, text);
CREATE FUNCTION "public"."pgp_pub_encrypt_bytea"(bytea, bytea, text)
  RETURNS "pg_catalog"."bytea" AS '$libdir/pgcrypto', 'pgp_pub_encrypt_bytea'
  LANGUAGE c VOLATILE STRICT
  COST 1;
ALTER FUNCTION "public"."pgp_pub_encrypt_bytea"(bytea, bytea, text) OWNER TO "postgres";

-- ----------------------------
-- Function structure for pgp_sym_decrypt
-- ----------------------------
DROP FUNCTION IF EXISTS "public"."pgp_sym_decrypt"(bytea, text, text);
CREATE FUNCTION "public"."pgp_sym_decrypt"(bytea, text, text)
  RETURNS "pg_catalog"."text" AS '$libdir/pgcrypto', 'pgp_sym_decrypt_text'
  LANGUAGE c IMMUTABLE STRICT
  COST 1;
ALTER FUNCTION "public"."pgp_sym_decrypt"(bytea, text, text) OWNER TO "postgres";

-- ----------------------------
-- Function structure for pgp_sym_decrypt
-- ----------------------------
DROP FUNCTION IF EXISTS "public"."pgp_sym_decrypt"(bytea, text);
CREATE FUNCTION "public"."pgp_sym_decrypt"(bytea, text)
  RETURNS "pg_catalog"."text" AS '$libdir/pgcrypto', 'pgp_sym_decrypt_text'
  LANGUAGE c IMMUTABLE STRICT
  COST 1;
ALTER FUNCTION "public"."pgp_sym_decrypt"(bytea, text) OWNER TO "postgres";

-- ----------------------------
-- Function structure for pgp_sym_decrypt_bytea
-- ----------------------------
DROP FUNCTION IF EXISTS "public"."pgp_sym_decrypt_bytea"(bytea, text);
CREATE FUNCTION "public"."pgp_sym_decrypt_bytea"(bytea, text)
  RETURNS "pg_catalog"."bytea" AS '$libdir/pgcrypto', 'pgp_sym_decrypt_bytea'
  LANGUAGE c IMMUTABLE STRICT
  COST 1;
ALTER FUNCTION "public"."pgp_sym_decrypt_bytea"(bytea, text) OWNER TO "postgres";

-- ----------------------------
-- Function structure for pgp_sym_decrypt_bytea
-- ----------------------------
DROP FUNCTION IF EXISTS "public"."pgp_sym_decrypt_bytea"(bytea, text, text);
CREATE FUNCTION "public"."pgp_sym_decrypt_bytea"(bytea, text, text)
  RETURNS "pg_catalog"."bytea" AS '$libdir/pgcrypto', 'pgp_sym_decrypt_bytea'
  LANGUAGE c IMMUTABLE STRICT
  COST 1;
ALTER FUNCTION "public"."pgp_sym_decrypt_bytea"(bytea, text, text) OWNER TO "postgres";

-- ----------------------------
-- Function structure for pgp_sym_encrypt
-- ----------------------------
DROP FUNCTION IF EXISTS "public"."pgp_sym_encrypt"(text, text);
CREATE FUNCTION "public"."pgp_sym_encrypt"(text, text)
  RETURNS "pg_catalog"."bytea" AS '$libdir/pgcrypto', 'pgp_sym_encrypt_text'
  LANGUAGE c VOLATILE STRICT
  COST 1;
ALTER FUNCTION "public"."pgp_sym_encrypt"(text, text) OWNER TO "postgres";

-- ----------------------------
-- Function structure for pgp_sym_encrypt
-- ----------------------------
DROP FUNCTION IF EXISTS "public"."pgp_sym_encrypt"(text, text, text);
CREATE FUNCTION "public"."pgp_sym_encrypt"(text, text, text)
  RETURNS "pg_catalog"."bytea" AS '$libdir/pgcrypto', 'pgp_sym_encrypt_text'
  LANGUAGE c VOLATILE STRICT
  COST 1;
ALTER FUNCTION "public"."pgp_sym_encrypt"(text, text, text) OWNER TO "postgres";

-- ----------------------------
-- Function structure for pgp_sym_encrypt_bytea
-- ----------------------------
DROP FUNCTION IF EXISTS "public"."pgp_sym_encrypt_bytea"(bytea, text);
CREATE FUNCTION "public"."pgp_sym_encrypt_bytea"(bytea, text)
  RETURNS "pg_catalog"."bytea" AS '$libdir/pgcrypto', 'pgp_sym_encrypt_bytea'
  LANGUAGE c VOLATILE STRICT
  COST 1;
ALTER FUNCTION "public"."pgp_sym_encrypt_bytea"(bytea, text) OWNER TO "postgres";

-- ----------------------------
-- Function structure for pgp_sym_encrypt_bytea
-- ----------------------------
DROP FUNCTION IF EXISTS "public"."pgp_sym_encrypt_bytea"(bytea, text, text);
CREATE FUNCTION "public"."pgp_sym_encrypt_bytea"(bytea, text, text)
  RETURNS "pg_catalog"."bytea" AS '$libdir/pgcrypto', 'pgp_sym_encrypt_bytea'
  LANGUAGE c VOLATILE STRICT
  COST 1;
ALTER FUNCTION "public"."pgp_sym_encrypt_bytea"(bytea, text, text) OWNER TO "postgres";

-- ----------------------------
-- Function structure for update_updated_at_column
-- ----------------------------
DROP FUNCTION IF EXISTS "public"."update_updated_at_column"();
CREATE FUNCTION "public"."update_updated_at_column"()
  RETURNS "pg_catalog"."trigger" AS $BODY$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$BODY$
  LANGUAGE plpgsql VOLATILE
  COST 100;
ALTER FUNCTION "public"."update_updated_at_column"() OWNER TO "postgres";

-- ----------------------------
-- Function structure for uuid_v7
-- ----------------------------
DROP FUNCTION IF EXISTS "public"."uuid_v7"();
CREATE FUNCTION "public"."uuid_v7"()
  RETURNS "pg_catalog"."uuid" AS $BODY$
DECLARE
    unix_ts_ms BIGINT;
    rand_bytes BYTEA;
    uuid_bytes BYTEA;
    ts_high INTEGER;
    ts_mid INTEGER;
    ts_low INTEGER;
    byte_val INTEGER;
BEGIN
    -- 获取当前 UNIX 时间戳（毫秒）
    unix_ts_ms := (EXTRACT(EPOCH FROM clock_timestamp()) * 1000)::BIGINT;

    -- 生成 10 个随机字节
    rand_bytes := gen_random_bytes(10);

    -- 构建 16 字节的 UUID
    uuid_bytes := '\x00000000000000000000000000000000'::bytea;

    -- 设置前 6 个字节为时间戳（48 位）
    -- 字节 0-1: 时间戳的高 16 位
    byte_val := (unix_ts_ms >> 40) & 255;
    uuid_bytes := set_byte(uuid_bytes, 0, byte_val);
    byte_val := (unix_ts_ms >> 32) & 255;
    uuid_bytes := set_byte(uuid_bytes, 1, byte_val);

    -- 字节 2-3: 时间戳的中间 16 位
    byte_val := (unix_ts_ms >> 24) & 255;
    uuid_bytes := set_byte(uuid_bytes, 2, byte_val);
    byte_val := (unix_ts_ms >> 16) & 255;
    uuid_bytes := set_byte(uuid_bytes, 3, byte_val);

    -- 字节 4-5: 时间戳的低 16 位
    byte_val := (unix_ts_ms >> 8) & 255;
    uuid_bytes := set_byte(uuid_bytes, 4, byte_val);
    byte_val := unix_ts_ms & 255;
    uuid_bytes := set_byte(uuid_bytes, 5, byte_val);

    -- 字节 6: 版本位 (0x70 = 0111 0000) 和随机数的高 4 位
    byte_val := (get_byte(rand_bytes, 0) & 15) | 112;  -- 112 = 0x70 (7 << 4)
    uuid_bytes := set_byte(uuid_bytes, 6, byte_val);

    -- 字节 7-8: 随机数
    uuid_bytes := set_byte(uuid_bytes, 7, get_byte(rand_bytes, 1));
    uuid_bytes := set_byte(uuid_bytes, 8, get_byte(rand_bytes, 2));

    -- 字节 9: 变体位 (0x80 = 1000 0000) 和随机数的低 6 位
    byte_val := (get_byte(rand_bytes, 3) & 63) | 128;  -- 128 = 0x80
    uuid_bytes := set_byte(uuid_bytes, 9, byte_val);

    -- 字节 10-15: 剩余随机字节
    uuid_bytes := set_byte(uuid_bytes, 10, get_byte(rand_bytes, 4));
    uuid_bytes := set_byte(uuid_bytes, 11, get_byte(rand_bytes, 5));
    uuid_bytes := set_byte(uuid_bytes, 12, get_byte(rand_bytes, 6));
    uuid_bytes := set_byte(uuid_bytes, 13, get_byte(rand_bytes, 7));
    uuid_bytes := set_byte(uuid_bytes, 14, get_byte(rand_bytes, 8));
    uuid_bytes := set_byte(uuid_bytes, 15, get_byte(rand_bytes, 9));

    -- 转换为 UUID
    RETURN encode(uuid_bytes, 'hex')::uuid;
END;
$BODY$
  LANGUAGE plpgsql VOLATILE
  COST 100;
ALTER FUNCTION "public"."uuid_v7"() OWNER TO "postgres";

-- ----------------------------
-- Table structure for guardian_admin_roles
-- ----------------------------
DROP TABLE IF EXISTS "public"."guardian_admin_roles";
CREATE TABLE "public"."guardian_admin_roles" (
  "admin_id" uuid NOT NULL,
  "role_id" uuid NOT NULL,
  "created_at" timestamptz(6) NOT NULL DEFAULT now()
)
;
ALTER TABLE "public"."guardian_admin_roles" OWNER TO "postgres";
COMMENT ON COLUMN "public"."guardian_admin_roles"."admin_id" IS '管理员ID（逻辑关联 guardian_admins.id，不使用外键）';
COMMENT ON COLUMN "public"."guardian_admin_roles"."role_id" IS '角色ID（逻辑关联 guardian_roles.id，不使用外键）';
COMMENT ON COLUMN "public"."guardian_admin_roles"."created_at" IS '创建时间';
COMMENT ON TABLE "public"."guardian_admin_roles" IS '管理员角色关联表';

-- ----------------------------
-- Records of guardian_admin_roles
-- ----------------------------
BEGIN;
INSERT INTO "public"."guardian_admin_roles" ("admin_id", "role_id", "created_at") VALUES ('019bbbe1-9c3b-7bcd-ac94-9b0616086ed8', '019bc26b-aaad-7077-34a6-1c111265dd63', '2026-01-16 00:41:22.106306+08');
COMMIT;

-- ----------------------------
-- Table structure for guardian_admins
-- ----------------------------
DROP TABLE IF EXISTS "public"."guardian_admins";
CREATE TABLE "public"."guardian_admins" (
  "id" uuid NOT NULL DEFAULT uuid_v7(),
  "username" varchar(64) COLLATE "pg_catalog"."default" NOT NULL,
  "password_hash" varchar(255) COLLATE "pg_catalog"."default" NOT NULL,
  "two_fa_secret" text COLLATE "pg_catalog"."default",
  "is_super_admin" bool DEFAULT false,
  "status" int2 DEFAULT 1,
  "last_login_at" timestamptz(6),
  "login_attempts" int4 DEFAULT 0,
  "locked_until" timestamptz(6),
  "created_at" timestamptz(6) NOT NULL DEFAULT now(),
  "updated_at" timestamptz(6) NOT NULL DEFAULT now()
)
;
ALTER TABLE "public"."guardian_admins" OWNER TO "postgres";
COMMENT ON COLUMN "public"."guardian_admins"."id" IS '管理员ID（UUIDv7）';
COMMENT ON COLUMN "public"."guardian_admins"."username" IS '用户名';
COMMENT ON COLUMN "public"."guardian_admins"."password_hash" IS 'argon2密码哈希';
COMMENT ON COLUMN "public"."guardian_admins"."two_fa_secret" IS '2FA密钥（ChaCha20加密存储）';
COMMENT ON COLUMN "public"."guardian_admins"."is_super_admin" IS '是否超级管理员';
COMMENT ON COLUMN "public"."guardian_admins"."status" IS '状态：1=正常，0=禁用';
COMMENT ON COLUMN "public"."guardian_admins"."last_login_at" IS '最后登录时间';
COMMENT ON COLUMN "public"."guardian_admins"."login_attempts" IS '登录失败尝试次数';
COMMENT ON COLUMN "public"."guardian_admins"."locked_until" IS '锁定截止时间';
COMMENT ON COLUMN "public"."guardian_admins"."created_at" IS '创建时间';
COMMENT ON COLUMN "public"."guardian_admins"."updated_at" IS '更新时间';
COMMENT ON TABLE "public"."guardian_admins" IS '管理员表';

-- ----------------------------
-- Records of guardian_admins
-- ----------------------------
BEGIN;
INSERT INTO "public"."guardian_admins" ("id", "username", "password_hash", "two_fa_secret", "is_super_admin", "status", "last_login_at", "login_attempts", "locked_until", "created_at", "updated_at") VALUES ('019bbbe1-9c3b-7bcd-ac94-9b0616086ed8', 'guardian', '$argon2id$v=19$m=19456,t=2,p=1$7KszMmwG69nCXa+uXpH6bw$f67x/bi8Nxjpx3y1a6nBaVkfscHFAnLVCaM2IIk4a6I', NULL, 't', 1, '2026-01-14 19:29:26.185307+08', 0, NULL, '2026-01-14 17:41:23.642658+08', '2026-01-14 21:51:25.303372+08');
COMMIT;

-- ----------------------------
-- Table structure for guardian_audit_logs
-- ----------------------------
DROP TABLE IF EXISTS "public"."guardian_audit_logs";
CREATE TABLE "public"."guardian_audit_logs" (
  "id" uuid NOT NULL DEFAULT uuid_v7(),
  "trace_id" varchar(64) COLLATE "pg_catalog"."default",
  "admin_id" uuid,
  "username" varchar(64) COLLATE "pg_catalog"."default",
  "action" varchar(32) COLLATE "pg_catalog"."default" NOT NULL,
  "resource" varchar(255) COLLATE "pg_catalog"."default" NOT NULL,
  "method" varchar(10) COLLATE "pg_catalog"."default" NOT NULL,
  "params" jsonb,
  "result" jsonb,
  "status_code" int4 NOT NULL,
  "ip_address" inet,
  "user_agent" text COLLATE "pg_catalog"."default",
  "duration_ms" int4 NOT NULL,
  "created_at" timestamptz(6) NOT NULL DEFAULT now()
)
;
ALTER TABLE "public"."guardian_audit_logs" OWNER TO "postgres";
COMMENT ON COLUMN "public"."guardian_audit_logs"."id" IS '日志ID（UUIDv7）';
COMMENT ON COLUMN "public"."guardian_audit_logs"."trace_id" IS '请求追踪ID（用于关联同一请求的多个操作）';
COMMENT ON COLUMN "public"."guardian_audit_logs"."admin_id" IS '操作管理员ID（逻辑关联，不使用外键）';
COMMENT ON COLUMN "public"."guardian_audit_logs"."username" IS '操作用户名';
COMMENT ON COLUMN "public"."guardian_audit_logs"."action" IS '操作类型：login/logout/create/update/delete等';
COMMENT ON COLUMN "public"."guardian_audit_logs"."resource" IS '操作资源：如/admins/123';
COMMENT ON COLUMN "public"."guardian_audit_logs"."method" IS 'HTTP方法：GET/POST/PUT/DELETE';
COMMENT ON COLUMN "public"."guardian_audit_logs"."params" IS '请求参数（JSONB格式）';
COMMENT ON COLUMN "public"."guardian_audit_logs"."result" IS '操作结果（JSONB格式）';
COMMENT ON COLUMN "public"."guardian_audit_logs"."status_code" IS 'HTTP状态码';
COMMENT ON COLUMN "public"."guardian_audit_logs"."ip_address" IS '客户端IP地址';
COMMENT ON COLUMN "public"."guardian_audit_logs"."user_agent" IS '客户端User-Agent';
COMMENT ON COLUMN "public"."guardian_audit_logs"."duration_ms" IS '请求耗时（毫秒）';
COMMENT ON COLUMN "public"."guardian_audit_logs"."created_at" IS '创建时间';
COMMENT ON TABLE "public"."guardian_audit_logs" IS '审计日志表';

-- ----------------------------
-- Records of guardian_audit_logs
-- ----------------------------
BEGIN;
COMMIT;

-- ----------------------------
-- Table structure for guardian_permissions
-- ----------------------------
DROP TABLE IF EXISTS "public"."guardian_permissions";
CREATE TABLE "public"."guardian_permissions" (
  "id" uuid NOT NULL DEFAULT uuid_v7(),
  "code" varchar(64) COLLATE "pg_catalog"."default" NOT NULL,
  "name" varchar(128) COLLATE "pg_catalog"."default" NOT NULL,
  "description" text COLLATE "pg_catalog"."default",
  "resource_type" varchar(32) COLLATE "pg_catalog"."default" NOT NULL,
  "http_method" varchar(10) COLLATE "pg_catalog"."default",
  "resource_path" varchar(255) COLLATE "pg_catalog"."default",
  "parent_id" uuid,
  "sort_order" int4 DEFAULT 0,
  "is_system" bool DEFAULT false,
  "created_at" timestamptz(6) NOT NULL DEFAULT now(),
  "updated_at" timestamptz(6) NOT NULL DEFAULT now()
)
;
ALTER TABLE "public"."guardian_permissions" OWNER TO "postgres";
COMMENT ON COLUMN "public"."guardian_permissions"."id" IS '权限ID（UUIDv7）';
COMMENT ON COLUMN "public"."guardian_permissions"."code" IS '权限代码（唯一）';
COMMENT ON COLUMN "public"."guardian_permissions"."name" IS '权限名称';
COMMENT ON COLUMN "public"."guardian_permissions"."description" IS '权限描述';
COMMENT ON COLUMN "public"."guardian_permissions"."resource_type" IS '资源类型：api/menu/button';
COMMENT ON COLUMN "public"."guardian_permissions"."http_method" IS 'HTTP方法：GET/POST/PUT/DELETE等';
COMMENT ON COLUMN "public"."guardian_permissions"."resource_path" IS '资源路径';
COMMENT ON COLUMN "public"."guardian_permissions"."parent_id" IS '父权限ID（逻辑关联，不使用外键）';
COMMENT ON COLUMN "public"."guardian_permissions"."sort_order" IS '排序序号';
COMMENT ON COLUMN "public"."guardian_permissions"."is_system" IS '是否系统内置权限（不可删除）';
COMMENT ON COLUMN "public"."guardian_permissions"."created_at" IS '创建时间';
COMMENT ON COLUMN "public"."guardian_permissions"."updated_at" IS '更新时间';
COMMENT ON TABLE "public"."guardian_permissions" IS '权限表';

-- ----------------------------
-- Records of guardian_permissions
-- ----------------------------
BEGIN;
COMMIT;

-- ----------------------------
-- Table structure for guardian_role_permissions
-- ----------------------------
DROP TABLE IF EXISTS "public"."guardian_role_permissions";
CREATE TABLE "public"."guardian_role_permissions" (
  "role_id" uuid NOT NULL,
  "permission_id" uuid NOT NULL,
  "created_at" timestamptz(6) NOT NULL DEFAULT now()
)
;
ALTER TABLE "public"."guardian_role_permissions" OWNER TO "postgres";
COMMENT ON COLUMN "public"."guardian_role_permissions"."role_id" IS '角色ID（逻辑关联 guardian_roles.id，不使用外键）';
COMMENT ON COLUMN "public"."guardian_role_permissions"."permission_id" IS '权限ID（逻辑关联 guardian_permissions.id，不使用外键）';
COMMENT ON COLUMN "public"."guardian_role_permissions"."created_at" IS '创建时间';
COMMENT ON TABLE "public"."guardian_role_permissions" IS '角色权限关联表';

-- ----------------------------
-- Records of guardian_role_permissions
-- ----------------------------
BEGIN;
COMMIT;

-- ----------------------------
-- Table structure for guardian_roles
-- ----------------------------
DROP TABLE IF EXISTS "public"."guardian_roles";
CREATE TABLE "public"."guardian_roles" (
  "id" uuid NOT NULL DEFAULT uuid_v7(),
  "code" varchar(32) COLLATE "pg_catalog"."default" NOT NULL,
  "name" varchar(64) COLLATE "pg_catalog"."default" NOT NULL,
  "description" text COLLATE "pg_catalog"."default",
  "is_system" bool DEFAULT false,
  "created_at" timestamptz(6) NOT NULL DEFAULT now(),
  "updated_at" timestamptz(6) NOT NULL DEFAULT now()
)
;
ALTER TABLE "public"."guardian_roles" OWNER TO "postgres";
COMMENT ON COLUMN "public"."guardian_roles"."id" IS '角色ID（UUIDv7）';
COMMENT ON COLUMN "public"."guardian_roles"."code" IS '角色代码（唯一）';
COMMENT ON COLUMN "public"."guardian_roles"."name" IS '角色名称';
COMMENT ON COLUMN "public"."guardian_roles"."description" IS '角色描述';
COMMENT ON COLUMN "public"."guardian_roles"."is_system" IS '是否系统内置角色（不可删除）';
COMMENT ON COLUMN "public"."guardian_roles"."created_at" IS '创建时间';
COMMENT ON COLUMN "public"."guardian_roles"."updated_at" IS '更新时间';
COMMENT ON TABLE "public"."guardian_roles" IS '角色表';

-- ----------------------------
-- Records of guardian_roles
-- ----------------------------
BEGIN;
INSERT INTO "public"."guardian_roles" ("id", "code", "name", "description", "is_system", "created_at", "updated_at") VALUES ('019bc26b-aaad-7077-34a6-1c111265dd63', '019bc1498add72f074bf0429e7165416', 'SuperAdmin', '系统默认超级管理员权限', 't', '2026-01-16 00:09:54.591761+08', '2026-01-16 00:40:39.833363+08');
COMMIT;

-- ----------------------------
-- Table structure for guardian_systeminfo
-- ----------------------------
DROP TABLE IF EXISTS "public"."guardian_systeminfo";
CREATE TABLE "public"."guardian_systeminfo" (
  "id" uuid NOT NULL DEFAULT uuid_v7(),
  "cpu_count" int4,
  "cpu_total_load" numeric(5,2),
  "memory_used" int8,
  "memory_total" int8,
  "disk_used" int8,
  "disk_total" int8,
  "network_upload" int8,
  "network_download" int8,
  "created_at" timestamptz(6) NOT NULL DEFAULT now()
)
;
ALTER TABLE "public"."guardian_systeminfo" OWNER TO "postgres";
COMMENT ON COLUMN "public"."guardian_systeminfo"."id" IS '记录ID（UUIDv7）';
COMMENT ON COLUMN "public"."guardian_systeminfo"."cpu_count" IS 'CPU核心数';
COMMENT ON COLUMN "public"."guardian_systeminfo"."cpu_total_load" IS 'CPU总使用率（%）';
COMMENT ON COLUMN "public"."guardian_systeminfo"."memory_used" IS '已使用内存（字节）';
COMMENT ON COLUMN "public"."guardian_systeminfo"."memory_total" IS '总内存（字节）';
COMMENT ON COLUMN "public"."guardian_systeminfo"."disk_used" IS '已使用磁盘空间（字节）';
COMMENT ON COLUMN "public"."guardian_systeminfo"."disk_total" IS '总磁盘空间（字节）';
COMMENT ON COLUMN "public"."guardian_systeminfo"."network_upload" IS '网络上传量（字节）';
COMMENT ON COLUMN "public"."guardian_systeminfo"."network_download" IS '网络下载量（字节）';
COMMENT ON COLUMN "public"."guardian_systeminfo"."created_at" IS '记录创建时间';
COMMENT ON TABLE "public"."guardian_systeminfo" IS '系统监控信息表';

-- ----------------------------
-- Records of guardian_systeminfo
-- ----------------------------
BEGIN;
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc0d7-485b-7e6c-aca0-b261aa76bff1', 8, 23.30, 5340725248, 17179869184, 12262895616, 494353338368, 0, 0, '2026-01-15 16:48:12.879672+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc0d8-2c3c-7ee3-87af-fcfe5265f0b5', 8, 13.40, 5332893696, 17179869184, 12262895616, 494353338368, 0, 0, '2026-01-15 16:49:11.217326+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc0d8-5036-7ff0-91ae-2487827cef58', 8, 12.20, 5351899136, 17179869184, 12262895616, 494353338368, 0, 0, '2026-01-15 16:49:20.43084+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc0d8-5cbe-73d3-a3bb-c1853d36f9fb', 8, 23.60, 5352095744, 17179869184, 12262895616, 494353338368, 0, 0, '2026-01-15 16:49:23.642824+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc0d8-66a8-7605-5ebc-afeb9afb229c', 8, 18.60, 5344411648, 17179869184, 12262895616, 494353338368, 0, 0, '2026-01-15 16:49:26.175303+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc0d8-85a5-76cb-bd86-a37fbbee3112', 8, 14.50, 5321261056, 17179869184, 12262895616, 494353338368, 0, 0, '2026-01-15 16:49:34.106971+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc0d8-8d63-7574-3798-30bb3c996eae', 8, 13.40, 5332140032, 17179869184, 12262895616, 494353338368, 0, 0, '2026-01-15 16:49:36.091721+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc0fc-a35d-74c9-bcb0-79acdb6525d3', 8, 13.00, 5926502400, 17179869184, 12262895616, 494353338368, 233472, 269312, '2026-01-15 17:29:01.010144+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc0fd-8db0-78e9-07ae-ba354217b992', 8, 13.50, 6008438784, 17179869184, 12262895616, 494353338368, 280576, 335872, '2026-01-15 17:30:01.00825+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc0fe-7813-7104-6488-970cbcea070e', 8, 18.40, 6018498560, 17179869184, 12262895616, 494353338368, 384000, 453632, '2026-01-15 17:31:01.011173+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc0ff-6274-790a-6287-55e1b7dbf8f5', 8, 36.50, 6096044032, 17179869184, 12262895616, 494353338368, 292864, 307200, '2026-01-15 17:32:01.011389+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc100-4cd4-7720-7cbb-dac60da4c405', 8, 20.10, 5938970624, 17179869184, 12262895616, 494353338368, 364544, 870400, '2026-01-15 17:33:01.011469+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc101-3735-7353-fea0-65fb67f7be42', 8, 18.70, 5757829120, 17179869184, 12262895616, 494353338368, 327680, 276480, '2026-01-15 17:34:01.012649+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc102-2194-7854-149a-ed515c94a155', 8, 33.70, 5902974976, 17179869184, 12262895616, 494353338368, 878592, 1167360, '2026-01-15 17:35:01.01211+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc103-0bf4-7478-0bbe-360fe145eae2', 8, 28.60, 5862064128, 17179869184, 12262895616, 494353338368, 684032, 431104, '2026-01-15 17:36:01.011414+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc103-f650-7ec4-64af-9c54c4b55a65', 8, 38.10, 5891145728, 17179869184, 12262895616, 494353338368, 445440, 549888, '2026-01-15 17:37:01.007861+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc104-e0b9-79bb-6186-57a9c563fca1', 8, 29.60, 5711200256, 17179869184, 12262895616, 494353338368, 4082688, 3852288, '2026-01-15 17:38:01.016256+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc105-cb15-7df6-e5a1-a5bb3f2920eb', 8, 30.40, 5408194560, 17179869184, 12262895616, 494353338368, 593920, 287744, '2026-01-15 17:39:01.012524+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc106-b571-7dd6-568c-c022a726cc38', 8, 21.50, 5563973632, 17179869184, 12262895616, 494353338368, 3096576, 2894848, '2026-01-15 17:40:01.008353+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc107-9fde-7a11-058f-a0c96b53c3e0', 8, 16.70, 5334892544, 17179869184, 12262895616, 494353338368, 1263616, 1379328, '2026-01-15 17:41:01.021787+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc108-8a39-7fe4-b5a1-a345b131a6d2', 8, 24.00, 5341560832, 17179869184, 12262895616, 494353338368, 598016, 667648, '2026-01-15 17:42:01.016297+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc109-7492-76a9-6f90-b74dc30b1d22', 8, 26.40, 5377376256, 17179869184, 12262895616, 494353338368, 878592, 760832, '2026-01-15 17:43:01.00998+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc10a-5eed-7522-78b1-047496fb77b9', 8, 20.00, 5318508544, 17179869184, 12262895616, 494353338368, 729088, 676864, '2026-01-15 17:44:01.004631+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc10b-494e-7289-6794-c8af0ff7ffc4', 8, 17.30, 5310595072, 17179869184, 12262895616, 494353338368, 376832, 423936, '2026-01-15 17:45:01.006142+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc10c-33b2-7e19-ecbb-706185c33277', 8, 13.70, 5372116992, 17179869184, 12262895616, 494353338368, 611328, 681984, '2026-01-15 17:46:01.009716+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc10d-1e12-7d46-e086-a9a63db0c4e0', 8, 20.00, 5294587904, 17179869184, 12262895616, 494353338368, 777216, 472064, '2026-01-15 17:47:01.009618+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc10e-0875-73cd-af98-d846437befae', 8, 27.00, 5396365312, 17179869184, 12262895616, 494353338368, 1155072, 350208, '2026-01-15 17:48:01.012482+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc10e-f2d3-7e86-a88a-f4b731c12721', 8, 16.40, 5268144128, 17179869184, 12262895616, 494353338368, 2546688, 1818624, '2026-01-15 17:49:01.010253+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc10f-dd30-7e32-839b-348384511844', 8, 24.40, 5386715136, 17179869184, 12262895616, 494353338368, 990208, 1059840, '2026-01-15 17:50:01.007822+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc110-c791-7797-ed96-c33f75bdf7b5', 8, 25.30, 5170741248, 17179869184, 12262895616, 494353338368, 2589696, 797696, '2026-01-15 17:51:01.007946+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc111-b1ef-7a48-c8ae-964c7019567e', 8, 35.00, 5063737344, 17179869184, 12262895616, 494353338368, 1232896, 680960, '2026-01-15 17:52:01.007134+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc112-9c50-7221-1eac-bfbc5f04dcde', 8, 34.30, 5115543552, 17179869184, 12262895616, 494353338368, 2434048, 574464, '2026-01-15 17:53:01.007771+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc113-86b3-75cc-c497-4e4abff95914', 8, 30.00, 5212979200, 17179869184, 12262895616, 494353338368, 2655232, 1314816, '2026-01-15 17:54:01.010204+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc114-7112-7afd-909b-9e5dd2391c02', 8, 27.20, 5409849344, 17179869184, 12262895616, 494353338368, 2057216, 813056, '2026-01-15 17:55:01.009765+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc115-5b75-7eec-4189-a858698ecdbc', 8, 27.00, 5355454464, 17179869184, 12262895616, 494353338368, 1708032, 675840, '2026-01-15 17:56:01.012553+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc116-45d2-7ba7-d9a4-c655c05fe655', 8, 42.80, 5359058944, 17179869184, 12262895616, 494353338368, 2753536, 510976, '2026-01-15 17:57:01.009515+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc117-3033-7856-bb91-7c4974dae68c', 8, 100.00, 4986568704, 17179869184, 12262895616, 494353338368, 5323776, 4612096, '2026-01-15 17:58:01.010197+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc118-1a98-7af2-ae8a-f41dc0f27e79', 8, 45.60, 5274566656, 17179869184, 12262895616, 494353338368, 1755136, 1042432, '2026-01-15 17:59:01.015301+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc119-04f4-7d84-25bd-02d6755fdb48', 8, 32.80, 5243731968, 17179869184, 12262895616, 494353338368, 1632256, 628736, '2026-01-15 18:00:01.011278+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc119-ef57-7b0a-37a1-1d177058c574', 8, 100.00, 5058445312, 17179869184, 12262895616, 494353338368, 5249024, 2771968, '2026-01-15 18:01:01.014157+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc11a-d9ba-72f7-ea82-c3619773caa3', 8, 29.50, 5259608064, 17179869184, 12262895616, 494353338368, 3512320, 3000320, '2026-01-15 18:02:01.017567+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc11b-c411-7f5e-42b3-3ccd52320a2d', 8, 45.10, 4887560192, 17179869184, 12262895616, 494353338368, 3415040, 2821120, '2026-01-15 18:03:01.008216+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc11c-ae6d-7e97-cba2-a6510067b119', 8, 28.90, 5478989824, 17179869184, 12262895616, 494353338368, 1794048, 940032, '2026-01-15 18:04:01.004733+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc11d-98d3-7ab4-cdbe-6565b4c80e3b', 8, 28.80, 5262131200, 17179869184, 12262895616, 494353338368, 1881088, 1069056, '2026-01-15 18:05:01.010832+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc11e-8330-78a1-0fbf-9d335d2ac0be', 8, 27.80, 5172051968, 17179869184, 12262895616, 494353338368, 2101248, 783360, '2026-01-15 18:06:01.007247+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc11f-6d8c-7359-9ca6-a35d4f209aa0', 8, 29.80, 5243158528, 17179869184, 12262895616, 494353338368, 2810880, 613376, '2026-01-15 18:07:01.004288+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc120-57f8-79f6-bc95-edb9b0a360d6', 8, 47.60, 5143773184, 17179869184, 12262895616, 494353338368, 1519616, 613376, '2026-01-15 18:08:01.015281+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc121-4251-7d33-2596-db240451e4f0', 8, 19.50, 5339594752, 17179869184, 12262895616, 494353338368, 2956288, 680960, '2026-01-15 18:09:01.009054+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc122-2cb3-7bd9-2c8c-11d88ebfc28f', 8, 20.80, 5110546432, 17179869184, 12262895616, 494353338368, 561152, 600064, '2026-01-15 18:10:01.010372+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc123-1714-7d0f-03ab-63ca8cc3c3f0', 8, 24.30, 5420728320, 17179869184, 12262895616, 494353338368, 288768, 350208, '2026-01-15 18:11:01.011606+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc124-0175-7ef1-679b-74dc8bf6827a', 8, 21.80, 5866717184, 17179869184, 12262895616, 494353338368, 760832, 1009664, '2026-01-15 18:12:01.012195+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc124-ebd1-7498-42b5-e23b51ca10f1', 8, 40.10, 4986781696, 17179869184, 12262895616, 494353338368, 529408, 587776, '2026-01-15 18:13:01.008244+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc125-d632-75e8-5ba3-0d07da112e7a', 8, 19.50, 4991860736, 17179869184, 12262895616, 494353338368, 525312, 524288, '2026-01-15 18:14:01.010175+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc126-c091-7d29-4ba4-0ea91aad42ce', 8, 17.00, 5177245696, 17179869184, 12262895616, 494353338368, 286720, 316416, '2026-01-15 18:15:01.0091+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc127-aaed-7df0-4aaf-17ff7d6493e3', 8, 34.70, 5256036352, 17179869184, 12262895616, 494353338368, 538624, 580608, '2026-01-15 18:16:01.004925+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc128-9553-7a47-c298-cc35f7e0da98', 8, 50.60, 5325783040, 17179869184, 12262895616, 494353338368, 728064, 739328, '2026-01-15 18:17:01.010523+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc129-7fb3-7361-aa86-e89db364fc8d', 8, 21.00, 5185142784, 17179869184, 12262895616, 494353338368, 445440, 466944, '2026-01-15 18:18:01.010753+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc12a-6a11-7d34-e39f-ab33a5ba5a57', 8, 15.60, 5373657088, 17179869184, 12262895616, 494353338368, 150528, 181248, '2026-01-15 18:19:01.009113+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc12b-5474-732e-60aa-2b323e1f5088', 8, 21.40, 5438717952, 17179869184, 12262895616, 494353338368, 273408, 347136, '2026-01-15 18:20:01.011516+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc12c-3ed4-7e59-7a99-3a3c66dcf226', 8, 15.80, 5198577664, 17179869184, 12262895616, 494353338368, 764928, 792576, '2026-01-15 18:21:01.011328+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc12d-2938-7581-9087-55dd86a74185', 8, 22.80, 5179506688, 17179869184, 12262895616, 494353338368, 484352, 499712, '2026-01-15 18:22:01.015415+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc12e-1390-7bbb-bdb3-76ba019f3b70', 8, 16.70, 5089722368, 17179869184, 12262895616, 494353338368, 734208, 773120, '2026-01-15 18:23:01.007956+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc12e-fdf0-704a-5fad-53f05955355f', 8, 19.10, 5068800000, 17179869184, 12262895616, 494353338368, 847872, 850944, '2026-01-15 18:24:01.008048+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc12f-e853-7c13-ebb1-61ebef6b8456', 8, 34.70, 5282021376, 17179869184, 12262895616, 494353338368, 590848, 612352, '2026-01-15 18:25:01.011062+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc130-d2b0-764d-62a3-3eb0b5582d09', 8, 27.30, 5134581760, 17179869184, 12262895616, 494353338368, 410624, 452608, '2026-01-15 18:26:01.007749+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc131-bd0f-706b-e386-e81087d160c6', 8, 19.80, 5218959360, 17179869184, 12262895616, 494353338368, 711680, 872448, '2026-01-15 18:27:01.006672+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc132-a773-7dca-c6a8-4ad6c7341afe', 8, 51.50, 5256757248, 17179869184, 12262895616, 494353338368, 599040, 614400, '2026-01-15 18:28:01.011179+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc133-91d6-77b7-b3a5-e020d314356b', 8, 38.40, 5190615040, 17179869184, 12262895616, 494353338368, 579584, 592896, '2026-01-15 18:29:01.013355+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc134-7c32-73d3-c8b8-f89cf382f7d8', 8, 27.20, 5373706240, 17179869184, 12262895616, 494353338368, 2597888, 4426752, '2026-01-15 18:30:01.009351+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc135-669a-7495-ac96-d308d110d7cd', 8, 18.50, 5264310272, 17179869184, 12262895616, 494353338368, 1790976, 2832384, '2026-01-15 18:31:01.016963+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc136-50f5-751b-11ad-6700acf57a88', 8, 22.30, 5162205184, 17179869184, 12262895616, 494353338368, 774144, 991232, '2026-01-15 18:32:01.012624+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc137-3b55-7a79-a09c-69f9871865d0', 8, 38.60, 5109202944, 17179869184, 12262895616, 494353338368, 2213888, 3537920, '2026-01-15 18:33:01.012381+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc138-25b7-7ea7-9f8a-eca447125add', 8, 19.30, 5159698432, 17179869184, 12262895616, 494353338368, 1340416, 1326080, '2026-01-15 18:34:01.013839+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc139-101b-7fc2-afb5-610dd9ea6550', 8, 32.30, 5194792960, 17179869184, 12262895616, 494353338368, 499712, 490496, '2026-01-15 18:35:01.018735+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc139-fa77-75be-118b-cb67088d8cc6', 8, 14.60, 4922408960, 17179869184, 12262895616, 494353338368, 1115136, 1161216, '2026-01-15 18:36:01.014736+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc13a-e4d3-74c6-c2a9-6142d355a45f', 8, 27.50, 5158174720, 17179869184, 12262895616, 494353338368, 574464, 592896, '2026-01-15 18:37:01.010512+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc13b-cf2e-7770-04b5-d0683e54d945', 8, 18.80, 5202411520, 17179869184, 12262895616, 494353338368, 1865728, 2081792, '2026-01-15 18:38:01.005631+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc13c-b996-7233-08a9-fc83d032330e', 8, 15.80, 5094277120, 17179869184, 12262895616, 494353338368, 441344, 494592, '2026-01-15 18:39:01.013619+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc13d-a3ef-72de-56b6-ee4cfc636f6a', 8, 34.60, 5202853888, 17179869184, 12262895616, 494353338368, 1591296, 2136064, '2026-01-15 18:40:01.007165+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc13e-8e59-7795-8a86-f0d3112b95c6', 8, 25.10, 5157208064, 17179869184, 12262895616, 494353338368, 3689472, 5988352, '2026-01-15 18:41:01.016961+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc13f-78b2-79e8-2da8-8df173b0e21a', 8, 23.10, 5089280000, 17179869184, 12262895616, 494353338368, 287744, 366592, '2026-01-15 18:42:01.010128+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc140-6319-794f-c98a-49fcdc90d9dd', 8, 32.90, 5360189440, 17179869184, 12262895616, 494353338368, 1542144, 1395712, '2026-01-15 18:43:01.015704+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc141-4d74-7a8a-1dbd-4e700df7cf6f', 8, 23.20, 5199511552, 17179869184, 12262895616, 494353338368, 875520, 846848, '2026-01-15 18:44:01.011991+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc142-37ce-7133-789b-4fde7ecebd5c', 8, 21.70, 5179228160, 17179869184, 12262895616, 494353338368, 432128, 434176, '2026-01-15 18:45:01.006053+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc143-2238-73c4-4499-3876b98a8480', 8, 21.60, 4922343424, 17179869184, 12262895616, 494353338368, 485376, 521216, '2026-01-15 18:46:01.01545+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc144-0c93-7d0b-08a3-7c6a214a9b19', 8, 22.40, 5020139520, 17179869184, 12262895616, 494353338368, 807936, 1033216, '2026-01-15 18:47:01.011004+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc144-f6f5-7489-c7a7-4e362eee2edd', 8, 23.20, 4918181888, 17179869184, 12262895616, 494353338368, 1000448, 1162240, '2026-01-15 18:48:01.012669+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc145-e152-7a9c-9daa-6c558fa2a3a7', 8, 22.30, 4973953024, 17179869184, 12262895616, 494353338368, 354304, 368640, '2026-01-15 18:49:01.009947+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc146-cbb3-78a5-6a9b-916cf18aa8f0', 8, 21.80, 5071405056, 17179869184, 12262895616, 494353338368, 816128, 847872, '2026-01-15 18:50:01.010554+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc147-b612-7258-6496-402f4039e423', 8, 18.80, 5160140800, 17179869184, 12262895616, 494353338368, 580608, 632832, '2026-01-15 18:51:01.009407+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc148-a073-703a-19bb-2b25e53e51d8', 8, 37.00, 5063933952, 17179869184, 12262895616, 494353338368, 544768, 585728, '2026-01-15 18:52:01.010419+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc149-8add-72f0-74bf-0429e7165416', 8, 25.60, 5255593984, 17179869184, 12262895616, 494353338368, 856064, 1085440, '2026-01-15 18:53:01.02033+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc14a-7535-76fa-f382-a654c6c477b2', 8, 20.80, 5066293248, 17179869184, 12262895616, 494353338368, 837632, 962560, '2026-01-15 18:54:01.0122+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc14b-5f9f-79b4-8094-0d4bee6a44b9', 8, 100.00, 5037293568, 17179869184, 12262895616, 494353338368, 21996544, 41422848, '2026-01-15 18:55:01.022481+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc14c-49f3-7d0d-488b-7f274997d54a', 8, 20.90, 5400264704, 17179869184, 12262895616, 494353338368, 354304, 562176, '2026-01-15 18:56:01.010372+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc14d-3453-730a-87a2-c6f9cc1a83c0', 8, 25.60, 5378293760, 17179869184, 12262895616, 494353338368, 859136, 902144, '2026-01-15 18:57:01.010981+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc14e-1eb2-720d-e0a5-41aca2233040', 8, 20.10, 5509267456, 17179869184, 12262895616, 494353338368, 378880, 476160, '2026-01-15 18:58:01.009711+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc14f-0913-7e33-e894-6a2e2c5f7b38', 8, 32.10, 5559189504, 17179869184, 12262895616, 494353338368, 673792, 674816, '2026-01-15 18:59:01.010606+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc14f-f374-736f-7c87-b32077e87552', 8, 21.50, 5515689984, 17179869184, 12262895616, 494353338368, 929792, 1030144, '2026-01-15 19:00:01.011674+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc150-ddd4-7e05-c4a4-6ac85636ce67', 8, 27.20, 5161533440, 17179869184, 12262895616, 494353338368, 588800, 650240, '2026-01-15 19:01:01.011852+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc151-c833-7d66-39ad-bef40929265c', 8, 29.10, 5200723968, 17179869184, 12262895616, 494353338368, 389120, 414720, '2026-01-15 19:02:01.011356+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc152-b29b-7d88-968d-c92f3fa7f1a6', 8, 94.30, 5039620096, 17179869184, 12262895616, 494353338368, 2004992, 2189312, '2026-01-15 19:03:01.018184+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc153-9cf6-74f7-f39a-b0c70df6b716', 8, 45.30, 5296095232, 17179869184, 12262895616, 494353338368, 3191808, 3264512, '2026-01-15 19:04:01.013701+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc154-8751-7c37-a181-b9d24418f88e', 8, 15.00, 5637685248, 17179869184, 12262895616, 494353338368, 297984, 519168, '2026-01-15 19:05:01.00924+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc155-71b7-7011-abb7-f5f7f4581e47', 8, 26.50, 5534711808, 17179869184, 12262895616, 494353338368, 1538048, 1725440, '2026-01-15 19:06:01.014175+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc156-5c11-7aad-e697-7a68450ff3f4', 8, 44.40, 5380308992, 17179869184, 12262895616, 494353338368, 2991104, 3367936, '2026-01-15 19:07:01.008732+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc1e7-edc1-71ba-c399-701d1c0ed8ad', 8, 38.30, 5719048192, 17179869184, 12262895616, 494353338368, 131736576, 263045120, '2026-01-15 21:46:01.011056+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc1e8-d814-72c1-0da4-647d3efe1a73', 8, 37.60, 5597495296, 17179869184, 12262895616, 494353338368, 142490624, 286234624, '2026-01-15 21:47:01.010765+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc1e9-c26f-7df2-c393-266fb4313cf9', 8, 48.40, 5556305920, 17179869184, 12262895616, 494353338368, 141614080, 284377088, '2026-01-15 21:48:01.006443+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc1ea-acd6-77ea-9997-947130c6c460', 8, 45.00, 5669224448, 17179869184, 12262895616, 494353338368, 141845504, 284417024, '2026-01-15 21:49:01.014048+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc1eb-973f-7153-1daa-645acd73bac7', 8, 47.60, 5849317376, 17179869184, 12262895616, 494353338368, 143325184, 286380032, '2026-01-15 21:50:01.017719+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc1ec-8193-77cd-c7a5-fc1478c2f1d6', 8, 31.30, 5673304064, 17179869184, 12262895616, 494353338368, 142404608, 285579264, '2026-01-15 21:51:01.01032+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc1ed-6bf2-74e8-1c89-1fc057b1b703', 8, 43.70, 5644910592, 17179869184, 12262895616, 494353338368, 142218240, 284813312, '2026-01-15 21:52:01.009868+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc1ee-5664-7a27-a28f-005286dc788c', 8, 46.90, 5633114112, 17179869184, 12262895616, 494353338368, 144345088, 286398464, '2026-01-15 21:53:01.02777+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc1ef-40ba-7ac8-c997-c999e01b76e9', 8, 36.90, 6202867712, 17179869184, 12262895616, 494353338368, 142177280, 285839360, '2026-01-15 21:54:01.016242+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc1f0-2b16-72a2-d999-43edc571f073', 8, 37.90, 5534793728, 17179869184, 12262895616, 494353338368, 141774848, 285393920, '2026-01-15 21:55:01.013406+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc1f1-1571-7dd7-22a4-7a6d503fd884', 8, 37.80, 5500174336, 17179869184, 12262895616, 494353338368, 142618624, 286477312, '2026-01-15 21:56:01.008277+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc1f1-ffe5-7ff1-249c-0fba07299281', 8, 36.60, 5630787584, 17179869184, 12262895616, 494353338368, 141517824, 238096384, '2026-01-15 21:57:01.028372+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc1f2-ea46-74c8-708b-3748dc62b495', 8, 27.60, 5561729024, 17179869184, 12262895616, 494353338368, 141159424, 283794432, '2026-01-15 21:58:01.030217+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc1f3-d492-7ebf-d181-2e6ec91cf520', 8, 44.30, 5667913728, 17179869184, 12262895616, 494353338368, 8598528, 151212032, '2026-01-15 21:59:01.008667+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc1f4-bef5-7d86-e79e-9c18ee106dca', 8, 42.90, 5752160256, 17179869184, 12262895616, 494353338368, 146855936, 289009664, '2026-01-15 22:00:01.012769+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc1f5-a953-7e10-7da7-c4b01ba432b4', 8, 40.20, 5510758400, 17179869184, 12262895616, 494353338368, 142036992, 285148160, '2026-01-15 22:01:01.01058+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc1f6-93bc-7136-1d94-9b04812b4f98', 8, 35.00, 5788041216, 17179869184, 12262895616, 494353338368, 144358400, 286913536, '2026-01-15 22:02:01.015817+08');
INSERT INTO "public"."guardian_systeminfo" ("id", "cpu_count", "cpu_total_load", "memory_used", "memory_total", "disk_used", "disk_total", "network_upload", "network_download", "created_at") VALUES ('019bc1f7-7e1d-705b-4ea1-36d7f46383f9', 8, 99.10, 5557829632, 17179869184, 12262895616, 494353338368, 143453184, 285214720, '2026-01-15 22:03:01.020366+08');
COMMIT;

-- ----------------------------
-- Table structure for guardian_token_blacklist
-- ----------------------------
DROP TABLE IF EXISTS "public"."guardian_token_blacklist";
CREATE TABLE "public"."guardian_token_blacklist" (
  "id" uuid NOT NULL DEFAULT uuid_v7(),
  "token_id" varchar(128) COLLATE "pg_catalog"."default" NOT NULL,
  "expires_at" timestamptz(6) NOT NULL,
  "created_at" timestamptz(6) NOT NULL DEFAULT now()
)
;
ALTER TABLE "public"."guardian_token_blacklist" OWNER TO "postgres";
COMMENT ON COLUMN "public"."guardian_token_blacklist"."id" IS '记录ID（UUIDv7）';
COMMENT ON COLUMN "public"."guardian_token_blacklist"."token_id" IS '令牌标识（JWT的jti）';
COMMENT ON COLUMN "public"."guardian_token_blacklist"."expires_at" IS '过期时间';
COMMENT ON COLUMN "public"."guardian_token_blacklist"."created_at" IS '创建时间';
COMMENT ON TABLE "public"."guardian_token_blacklist" IS '令牌黑名单表';

-- ----------------------------
-- Records of guardian_token_blacklist
-- ----------------------------
BEGIN;
COMMIT;

-- ----------------------------
-- Indexes structure for table guardian_admin_roles
-- ----------------------------
CREATE INDEX "idx_guardian_admin_roles_admin_id" ON "public"."guardian_admin_roles" USING btree (
  "admin_id" "pg_catalog"."uuid_ops" ASC NULLS LAST
);
CREATE INDEX "idx_guardian_admin_roles_role_id" ON "public"."guardian_admin_roles" USING btree (
  "role_id" "pg_catalog"."uuid_ops" ASC NULLS LAST
);

-- ----------------------------
-- Primary Key structure for table guardian_admin_roles
-- ----------------------------
ALTER TABLE "public"."guardian_admin_roles" ADD CONSTRAINT "guardian_admin_roles_pkey" PRIMARY KEY ("admin_id", "role_id");

-- ----------------------------
-- Indexes structure for table guardian_admins
-- ----------------------------
CREATE INDEX "idx_guardian_admins_status" ON "public"."guardian_admins" USING btree (
  "status" "pg_catalog"."int2_ops" ASC NULLS LAST
);
CREATE INDEX "idx_guardian_admins_username" ON "public"."guardian_admins" USING btree (
  "username" COLLATE "pg_catalog"."default" "pg_catalog"."text_ops" ASC NULLS LAST
);

-- ----------------------------
-- Triggers structure for table guardian_admins
-- ----------------------------
CREATE TRIGGER "guardian_admins_updated_at" BEFORE UPDATE ON "public"."guardian_admins"
FOR EACH ROW
EXECUTE PROCEDURE "public"."update_updated_at_column"();

-- ----------------------------
-- Uniques structure for table guardian_admins
-- ----------------------------
ALTER TABLE "public"."guardian_admins" ADD CONSTRAINT "guardian_admins_username_key" UNIQUE ("username");

-- ----------------------------
-- Primary Key structure for table guardian_admins
-- ----------------------------
ALTER TABLE "public"."guardian_admins" ADD CONSTRAINT "guardian_admins_pkey" PRIMARY KEY ("id");

-- ----------------------------
-- Indexes structure for table guardian_audit_logs
-- ----------------------------
CREATE INDEX "idx_guardian_audit_logs_admin_id" ON "public"."guardian_audit_logs" USING btree (
  "admin_id" "pg_catalog"."uuid_ops" ASC NULLS LAST
);
CREATE INDEX "idx_guardian_audit_logs_created_at" ON "public"."guardian_audit_logs" USING btree (
  "created_at" "pg_catalog"."timestamptz_ops" ASC NULLS LAST
);
CREATE INDEX "idx_guardian_audit_logs_trace_id" ON "public"."guardian_audit_logs" USING btree (
  "trace_id" COLLATE "pg_catalog"."default" "pg_catalog"."text_ops" ASC NULLS LAST
);
CREATE INDEX "idx_guardian_audit_logs_username" ON "public"."guardian_audit_logs" USING btree (
  "username" COLLATE "pg_catalog"."default" "pg_catalog"."text_ops" ASC NULLS LAST
);

-- ----------------------------
-- Primary Key structure for table guardian_audit_logs
-- ----------------------------
ALTER TABLE "public"."guardian_audit_logs" ADD CONSTRAINT "guardian_audit_logs_pkey" PRIMARY KEY ("id");

-- ----------------------------
-- Indexes structure for table guardian_permissions
-- ----------------------------
CREATE INDEX "idx_guardian_permissions_code" ON "public"."guardian_permissions" USING btree (
  "code" COLLATE "pg_catalog"."default" "pg_catalog"."text_ops" ASC NULLS LAST
);
CREATE INDEX "idx_guardian_permissions_parent" ON "public"."guardian_permissions" USING btree (
  "parent_id" "pg_catalog"."uuid_ops" ASC NULLS LAST
);
CREATE INDEX "idx_guardian_permissions_resource" ON "public"."guardian_permissions" USING btree (
  "resource_type" COLLATE "pg_catalog"."default" "pg_catalog"."text_ops" ASC NULLS LAST,
  "resource_path" COLLATE "pg_catalog"."default" "pg_catalog"."text_ops" ASC NULLS LAST
);

-- ----------------------------
-- Triggers structure for table guardian_permissions
-- ----------------------------
CREATE TRIGGER "guardian_permissions_updated_at" BEFORE UPDATE ON "public"."guardian_permissions"
FOR EACH ROW
EXECUTE PROCEDURE "public"."update_updated_at_column"();

-- ----------------------------
-- Uniques structure for table guardian_permissions
-- ----------------------------
ALTER TABLE "public"."guardian_permissions" ADD CONSTRAINT "guardian_permissions_code_key" UNIQUE ("code");

-- ----------------------------
-- Primary Key structure for table guardian_permissions
-- ----------------------------
ALTER TABLE "public"."guardian_permissions" ADD CONSTRAINT "guardian_permissions_pkey" PRIMARY KEY ("id");

-- ----------------------------
-- Indexes structure for table guardian_role_permissions
-- ----------------------------
CREATE INDEX "idx_guardian_role_permissions_permission_id" ON "public"."guardian_role_permissions" USING btree (
  "permission_id" "pg_catalog"."uuid_ops" ASC NULLS LAST
);
CREATE INDEX "idx_guardian_role_permissions_role_id" ON "public"."guardian_role_permissions" USING btree (
  "role_id" "pg_catalog"."uuid_ops" ASC NULLS LAST
);

-- ----------------------------
-- Primary Key structure for table guardian_role_permissions
-- ----------------------------
ALTER TABLE "public"."guardian_role_permissions" ADD CONSTRAINT "guardian_role_permissions_pkey" PRIMARY KEY ("role_id", "permission_id");

-- ----------------------------
-- Indexes structure for table guardian_roles
-- ----------------------------
CREATE INDEX "idx_guardian_roles_code" ON "public"."guardian_roles" USING btree (
  "code" COLLATE "pg_catalog"."default" "pg_catalog"."text_ops" ASC NULLS LAST
);

-- ----------------------------
-- Triggers structure for table guardian_roles
-- ----------------------------
CREATE TRIGGER "guardian_roles_updated_at" BEFORE UPDATE ON "public"."guardian_roles"
FOR EACH ROW
EXECUTE PROCEDURE "public"."update_updated_at_column"();

-- ----------------------------
-- Uniques structure for table guardian_roles
-- ----------------------------
ALTER TABLE "public"."guardian_roles" ADD CONSTRAINT "guardian_roles_code_key" UNIQUE ("code");

-- ----------------------------
-- Primary Key structure for table guardian_roles
-- ----------------------------
ALTER TABLE "public"."guardian_roles" ADD CONSTRAINT "guardian_roles_pkey" PRIMARY KEY ("id");

-- ----------------------------
-- Indexes structure for table guardian_systeminfo
-- ----------------------------
CREATE INDEX "idx_guardian_systeminfo_created_at" ON "public"."guardian_systeminfo" USING btree (
  "created_at" "pg_catalog"."timestamptz_ops" ASC NULLS LAST
);

-- ----------------------------
-- Primary Key structure for table guardian_systeminfo
-- ----------------------------
ALTER TABLE "public"."guardian_systeminfo" ADD CONSTRAINT "guardian_systeminfo_pkey" PRIMARY KEY ("id");

-- ----------------------------
-- Indexes structure for table guardian_token_blacklist
-- ----------------------------
CREATE INDEX "idx_guardian_token_blacklist_expires_at" ON "public"."guardian_token_blacklist" USING btree (
  "expires_at" "pg_catalog"."timestamptz_ops" ASC NULLS LAST
);
CREATE INDEX "idx_guardian_token_blacklist_token_id" ON "public"."guardian_token_blacklist" USING btree (
  "token_id" COLLATE "pg_catalog"."default" "pg_catalog"."text_ops" ASC NULLS LAST
);

-- ----------------------------
-- Uniques structure for table guardian_token_blacklist
-- ----------------------------
ALTER TABLE "public"."guardian_token_blacklist" ADD CONSTRAINT "guardian_token_blacklist_token_id_key" UNIQUE ("token_id");

-- ----------------------------
-- Primary Key structure for table guardian_token_blacklist
-- ----------------------------
ALTER TABLE "public"."guardian_token_blacklist" ADD CONSTRAINT "guardian_token_blacklist_pkey" PRIMARY KEY ("id");
