#!/usr/bin/env python3
"""
Wrapper script that calls the main start_docs.py in the Reloaded directory with parameters.
"""

import subprocess
import sys
from pathlib import Path

def main():
    """Call the main start_docs.py script with prs-rs specific parameters."""
    # Get the directory where this script is located
    script_dir = Path(__file__).parent
    reloaded_script = script_dir / "docs" / "Reloaded" / "start_docs.py"
    
    if not reloaded_script.exists():
        print(f"Error: Could not find {reloaded_script}")
        sys.exit(1)
    
    # Run the main script with parameters for prs-rs
    print(f"Running prs-rs documentation setup from: {reloaded_script}")
    subprocess.run([
        sys.executable, 
        str(reloaded_script), 
        "--target-dir", str(script_dir),
        "--project-name", "prs-rs documentation"
    ], cwd=script_dir)

if __name__ == "__main__":
    main()