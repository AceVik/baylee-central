ALTER TABLE central_identity.gateway_tickets ADD COLUMN purpose text NOT NULL DEFAULT 'admit'
             CHECK (purpose IN ('admit', 'export', 'link'));
             ALTER TABLE central_identity.gateway_tickets ALTER COLUMN purpose DROP DEFAULT;
