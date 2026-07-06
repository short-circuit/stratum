"""MkDocs hooks — runs during build to inject dynamic values."""

import datetime


def on_pre_build(config, **kwargs):
    """Replace {year} placeholder in config values with current year."""
    year = str(datetime.date.today().year)
    for key in ("copyright",):
        if key in config and isinstance(config[key], str):
            config[key] = config[key].replace("{year}", year)
