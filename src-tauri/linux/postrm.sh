#!/bin/sh
# Dateiendung .mrpack und pinecone://-Links beim System anmelden.
command -v update-mime-database >/dev/null && update-mime-database /usr/share/mime || true
command -v update-desktop-database >/dev/null && update-desktop-database /usr/share/applications || true
exit 0
