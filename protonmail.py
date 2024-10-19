from selenium import webdriver
from selenium.webdriver.common.by import By
from selenium.webdriver.support.ui import WebDriverWait
from selenium.webdriver.support import expected_conditions as EC
import time
import random
import os
from dotenv import load_dotenv

load_dotenv()

PASSWORD = os.getenv("PROTONMAIL_PASSWORD")
if not PASSWORD:
    raise ValueError("PROTONMAIL_PASSWORD not found in .env file")


def login_to_protonmail():
    driver = webdriver.Chrome()
    wait = WebDriverWait(driver, 30)

    try:
        driver.get("https://account.proton.me/")

        username_input = wait.until(
            EC.presence_of_element_located((By.ID, "username"))
        )
        time.sleep(2)
        username_input.send_keys("ethsftee")

        password_input = wait.until(EC.presence_of_element_located((By.ID, "password")))
        time.sleep(2)
        password_input.send_keys(PASSWORD)
        password_input.submit()
        time.sleep(5)

        driver.get("https://account.proton.me/u/0/mail/account-password")
        time.sleep(2)
        change_password_button = wait.until(EC.presence_of_element_located((By.XPATH, "//button[contains(text(), 'Change password')]")))
        change_password_button.click()
        password_input = wait.until(EC.presence_of_element_located((By.ID, "password")))
        time.sleep(2)
        password_input.send_keys(PASSWORD)
        password_input.submit()
        time.sleep(5)

        new_password = ''.join(random.choices('abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@#$%^&*()_+-=', k=16))
        print(new_password)

        new_password_input = wait.until(EC.presence_of_element_located((By.ID, "newPassword")))
        new_password_input.send_keys(new_password)

        confirm_password_input = wait.until(EC.presence_of_element_located((By.ID, "confirmPassword")))
        confirm_password_input.send_keys(new_password)
        confirm_password_input.submit()

        time.sleep(15)

        
    
    except Exception as e:
        print(e)

if __name__ == "__main__":
    login_to_protonmail()
