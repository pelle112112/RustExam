import streamlit as st
import requests

# API URL for Rust backend
API_URL = "http://localhost:3000"

# Session state for login
if "logged_in" not in st.session_state:
    st.session_state.logged_in = False
if "username" not in st.session_state:
    st.session_state.username = ""

# Dummy credentials (just in case backend fails)
USER_CREDENTIALS = {
    "admin": "admin123", #Admin credentials
    
    "user": "password" #User credentials
}

def login(username, password):
    # Test connection with a get request
    response = requests.get(API_URL+"/hello/world")
    if response.status_code == 200:
        print("Connection successful")
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
            st.error("Invalid username or password.")
