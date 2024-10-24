import os
import sys
#os.environ['SE_CACHE_PATH'] = "/root/untrustedhost/"
import undetected_chromedriver as uc
import time
from selenium import webdriver
from selenium.webdriver import ChromeOptions, Keys
from selenium.webdriver.common.by import By
from selenium.webdriver.support import expected_conditions as EC
from selenium.webdriver.support.wait import WebDriverWait
import random
from dotenv import load_dotenv

load_dotenv()

PASSWORD = os.getenv("TWITTER_PASSWORD")
if not PASSWORD:
    raise ValueError("TWITTER_PASSWORD not found in .env file")

if 0:
    options = ChromeOptions()
    options.add_argument("--remote-debugging-pipe");
    options.add_argument("--start-maximized")
    options.add_argument("--headless")
    options.add_experimental_option("excludeSwitches", ["enable-automation"])
    options.binary_location = "/usr/bin/chromium"
    driver = webdriver.Chrome(options=options)
else:
    options = ChromeOptions()
    options.add_argument("--start-maximized")
    options.add_argument('--disable-dev-shm-usage')
    options.add_argument('--no-sandbox')
    driver = uc.Chrome(headless=True, use_subprocess=False, browser_executable_path='/usr/bin/chromium', options=options)

url = "https://twitter.com/i/flow/login"
driver.get(url)

username = WebDriverWait(driver, 20).until(EC.visibility_of_element_located((By.CSS_SELECTOR, 'input[autocomplete="username"]')))
print("username", file=sys.stderr)
username.send_keys("grouchy_mev")
username.send_keys(Keys.ENTER)

password = WebDriverWait(driver, 10).until(EC.visibility_of_element_located((By.CSS_SELECTOR, 'input[name="password"]')))
print("password", file=sys.stderr)
password.send_keys(PASSWORD)
password.send_keys(Keys.ENTER)

time.sleep(15)

new_password = ''.join(random.choices('abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@#$%^&*()_+-=', k=16))
print(new_password)

driver.get("https://x.com/settings/password")
time.sleep(2)
password_input = WebDriverWait(driver, 20).until(EC.visibility_of_element_located((By.NAME, "current_password")))
print("password", file=sys.stderr)
time.sleep(2)
password_input.send_keys(PASSWORD)

new_password_input = WebDriverWait(driver, 20).until(EC.visibility_of_element_located((By.NAME, "new_password")))
print("new password", file=sys.stderr)
new_password_input.send_keys(new_password)

confirm_password_input = WebDriverWait(driver, 20).until(EC.visibility_of_element_located((By.NAME, "password_confirmation")))
print("confirm", file=sys.stderr)
confirm_password_input.send_keys(new_password)

submit_button = WebDriverWait(driver, 20).until(EC.visibility_of_element_located((By.XPATH, '/html/body/div[1]/div/div/div[2]/main/div/div/div/section[2]/div[2]/div[3]/button')))
submit_button.click()

time.sleep(10)
