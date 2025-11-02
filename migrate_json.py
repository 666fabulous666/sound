#!/usr/bin/env python3
import json
import sys
from pathlib import Path

def migrate_node(node):
    """Migrate a TrackNode from old enum format to new struct format"""
    if "Group" in node:
        old_group = node["Group"]
        # Extract shared fields (with defaults)
        group_fields = {
            "id": old_group["id"],
            "muted": old_group["muted"],
            "collapsed": old_group["collapsed"],
            "children": [migrate_node(child) for child in old_group.get("children", [])],
        }
        if "not_generate_until" in old_group:
            group_fields["not_generate_until"] = old_group["not_generate_until"]

        result = {
            "name": old_group.get("name", ""),
            "proba": old_group.get("proba", 1.0),
            "volume": old_group.get("volume", 1.0),
            "pan": old_group.get("spacial", 0.5),  # Rename spacial to pan
            "hue": 0.0,  # New field
            "Group": group_fields,  # Keep the Group tag for serde
        }
        return result
    elif "Seq" in node:
        old_seq = node["Seq"]
        # Build the Seq fields (everything except shared fields)
        seq_fields = {}
        for key, value in old_seq.items():
            if key not in ["name", "proba", "volume", "spacial"]:
                seq_fields[key] = value

        # Extract shared fields and keep Seq tag
        result = {
            "name": old_seq.get("name", ""),
            "proba": old_seq.get("proba", 1.0),
            "volume": old_seq.get("volume", 1.0),
            "pan": old_seq.get("spacial", 0.5),  # Rename spacial to pan
            "hue": 0.0,  # New field
            "Seq": seq_fields,  # Keep the Seq tag for serde
        }
        return result
    else:
        # Already new format or unknown format
        return node

def migrate_file(input_path, output_path=None):
    """Migrate a JSON file"""
    if output_path is None:
        output_path = input_path

    with open(input_path, 'r') as f:
        data = json.load(f)

    # Migrate the seqs/root field
    if "seqs" in data:
        data["seqs"] = migrate_node(data["seqs"])

    with open(output_path, 'w') as f:
        json.dump(data, f, indent=2)

    print(f"Migrated: {input_path}")

if __name__ == "__main__":
    assets_dir = Path("/Users/fabienmathieu/Documents/prog/rust/synth/assets")
    json_files = list(assets_dir.glob("*.json"))

    for json_file in json_files:
        try:
            migrate_file(json_file)
        except Exception as e:
            print(f"Error migrating {json_file}: {e}")
            import traceback
            traceback.print_exc()
