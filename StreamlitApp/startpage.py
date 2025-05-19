import streamlit as st
import requests

# API URL for Rust backend
API_URL = "http://localhost:3000"

# Session state for login
if "token" not in st.session_state:
    st.session_state.token = False
if "username" not in st.session_state:
    st.session_state.username = ""

# Dummy credentials (just in case backend fails)
USER_CREDENTIALS = {
    "admin": "admin123", #Admin credentials
    
    "user": "password" #User credentials
}

def login(username, password):
    response = requests.post(API_URL+"/login", json={"username": username, "password": password})
    if response.status_code == 200:
        st.session_state.token = response.json().get("token")
    else :
        st.error(response.text)
    return response.status_code == 200

st.title("🔐 Login")

with st.form("login_form"):
    username = st.text_input("Username")
    password = st.text_input("Password", type="password")
    submit = st.form_submit_button("Login")

    if submit:
        if login(username, password):
            st.session_state.logged_in = True
            st.session_state.username = username
            st.success("Login successful!")
            st.info("Now navigate to 'Document Manager' from the sidebar.")
        else:
            del st.session_state.token
            del st.session_state.username
