-- Drop the workspaces table introduced in 0001.
--
-- Workspaces are no longer user-managed records. workspace-proxy maintains
-- an in-process registry populated by chan-tunneld at runtime, so
-- there is nothing left to persist.

DROP TABLE IF EXISTS workspaces;
