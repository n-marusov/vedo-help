FROM quay.io/keycloak/keycloak:26.1 AS base

# ------------------------------------------------------------------
# Custom KeyCloak image with realm auto-import and env var
# substitution for VEDO hub local development.
#
# The entrypoint reads realm-import.json, substitutes ${VAR}
# placeholders from environment variables, and imports the realm
# before starting KeyCloak in production mode.
# ------------------------------------------------------------------

USER root
RUN mkdir -p /opt/keycloak/data/import
COPY realm-import.json /opt/keycloak/data/import/realm-import.json.template

# Fix permissions so the keycloak user can read the template
RUN chown -R 1000:0 /opt/keycloak/data/import && \
    chmod 640 /opt/keycloak/data/import/realm-import.json.template

USER 1000

# Copy the bootstrap script
COPY --chown=1000:0 docker-entrypoint.sh /opt/keycloak/bin/docker-entrypoint.sh
RUN chmod +x /opt/keycloak/bin/docker-entrypoint.sh

ENTRYPOINT ["/opt/keycloak/bin/docker-entrypoint.sh"]
