import streamlit as st
import requests

# API URL for Rust backend
API_URL = "http://localhost:3000"

# Require login
if not st.session_state.get("logged_in", False):
    st.warning("Please log in first.")
    st.stop()

st.title("📄 Document Manager")

# File upload
uploaded_file = st.file_uploader("Upload a file")
if uploaded_file:
    files = {'file': uploaded_file.getvalue()}
    response = requests.post(f"{API_URL}/upload", files=files)

    if response.status_code == 200:
        st.success("File uploaded successfully.")
    else:
        st.error("File upload failed.")

# Fetch files from backend
response = requests.get(f"{API_URL}/files")
if response.status_code == 200:
    files = response.json()
    for file in files:
        st.download_button(label=f"Download {file['filename']}", data=file['file_url'])
else:
    st.error("Failed to fetch files.")
