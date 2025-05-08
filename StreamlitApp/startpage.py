import streamlit as st


# Login logic
if "logged_in" not in st.session_state:
    st.session_state.logged_in = False
if "username" not in st.session_state:
    st.session_state.username = ""

USER_CREDENTIALS = {
    "admin": "admin123",
    "user": "password"
}

def login(username, password):
    return USER_CREDENTIALS.get(username) == password

st.title("Login")

with st.form("login_form"):
    username = st.text_input("Username")
    password = st.text_input("Password", type="password")
    submit = st.form_submit_button("Login")

    if submit:
        if login(username, password):
            st.session_state.logged_in = True
            st.session_state.username = username
            st.success("Login successful!")
            st.info("Navigate to 'Document Manager' from the sidebar.")
        else:
            st.error("Invalid username or password.")
