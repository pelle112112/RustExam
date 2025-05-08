import streamlit as st


# Require login
if not st.session_state.get("logged_in", False):
    st.warning("Please log in from the start page.")
    st.stop()

st.title("Document Manager")

# Initialize file store
if "files" not in st.session_state:
    st.session_state.files = {}

# Upload files
uploaded_file = st.file_uploader("Upload a file")
if uploaded_file:
    st.session_state.files[uploaded_file.name] = uploaded_file.read()
    st.success(f"Uploaded: {uploaded_file.name}")

# List and download files
if st.session_state.files:
    st.subheader("Available Files")
    for filename, content in st.session_state.files.items():
        st.download_button(
            label=f"Download {filename}",
            data=content,
            file_name=filename,
        )
else:
    st.info("No files uploaded yet.")
