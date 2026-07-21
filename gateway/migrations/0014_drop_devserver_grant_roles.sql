-- A devserver grant is one binary, shell-equivalent authority. Viewer/editor
-- labels implied a containment boundary that the terminal data plane cannot
-- enforce.
ALTER TABLE devserver_grants DROP COLUMN role;
