-- Drop the drives table introduced in 0001.
--
-- Drives are no longer user-managed records. drive-proxy maintains
-- an in-process registry populated by chan-tunneld at runtime, so
-- there is nothing left to persist.

DROP TABLE IF EXISTS drives;
