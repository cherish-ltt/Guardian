/*
 Navicat Premium Dump SQL

 Source Server         : 本地postgreSQL
 Source Server Type    : PostgreSQL
 Source Server Version : 170007 (170007)
 Source Host           : 127.0.0.1:5432
 Source Catalog        : guardian_auth
 Source Schema         : public

 Target Server Type    : PostgreSQL
 Target Server Version : 170007 (170007)
 File Encoding         : 65001

 Date: 14/01/2026 21:51:37
*/


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
