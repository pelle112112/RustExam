import streamlit as st
import requests

# API URL for Rust backend
API_URL = "http://localhost:3000"

# Require login
if not st.session_state.get("logged_in", False):
    st.warning("Please log in first.")
    st.stop()

st.title("📄 Document Manager")

# ——————————————————————————————
# File upload
# ——————————————————————————————
uploaded_file = st.file_uploader("Upload a file")
if uploaded_file:
    files = {
        'file': (uploaded_file.name,
                 uploaded_file.getvalue(),
                 uploaded_file.type)
    }
    response = requests.post(f"{API_URL}/upload", files=files)
    if response.status_code == 200:
        st.success("File uploaded successfully.")
    else:
        st.error(f"Upload failed: {response.status_code}")

# ——————————————————————————————
# Fetch and list files
# ——————————————————————————————
st.header("Available Files")
resp = requests.get(f"{API_URL}/files")
if resp.status_code == 200:
    file_list = resp.json()  # should be a list of filenames (strings)
    if not file_list:
        st.info("No files found.")
    for filename in file_list:
        # For each filename, fetch its content and render a download button
        download_url = f"{API_URL}/download_file/{filename}"
        file_resp = requests.get(download_url)
        if file_resp.status_code == 200:
            st.download_button(
                label=f"Download {filename}",
                data=file_resp.content,
                file_name=filename
            )
        else:
            st.error(f"Failed to download {filename}: {file_resp.status_code}")
else:
    st.error(f"Failed to fetch file list: {resp.status_code}")
