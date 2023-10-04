# liveDashboard.py
# Display a live dashboard of the blockchain
# uses .env file in the parent directory to access environment variables
import psycopg2
import matplotlib.pyplot as plt
from dotenv import load_dotenv
import os
from matplotlib.animation import FuncAnimation
from datetime import datetime

PLOT_TIME_WINDOW_MINUTES = 5
AVERAGE_BLOCK_TIME_SECONDS = 6
NUMBER_OF_BLOCKS = int(PLOT_TIME_WINDOW_MINUTES * 60 / AVERAGE_BLOCK_TIME_SECONDS)
# NUMBER_OF_BLOCKS = 100
UPDATE_INTERVAL = 6000

# Load environment variables from the .env file in the parent directory
dotenv_path = os.path.join(os.path.dirname(__file__), "..", ".env")
load_dotenv(dotenv_path)

# Access environment variables
postgres_host = os.getenv("POSTGRES_HOST")
postgres_port = os.getenv("POSTGRES_PORT")
postgres_user = os.getenv("POSTGRES_USER")
postgres_password = os.getenv("POSTGRES_PASSWORD")
postgres_database = os.getenv("POSTGRES_DATABASE")

# Connect to the local PostgreSQL database
conn = psycopg2.connect(
    host=postgres_host,
    port=postgres_port,
    user=postgres_user,
    password=postgres_password,
    database=postgres_database
)

# Set the page size and the FIFO size
page_size = NUMBER_OF_BLOCKS
fifo_size = NUMBER_OF_BLOCKS

# Initialize empty lists for the X and Y data points
x_values_tx, y_values_tx = [], []
x_values_gas, y_values_gas = [], []
x_values_ts, y_values_ts = [], []
x_values_tps, y_values_tps = [], []
x_values_bt, y_values_bt = [], []
x_values_size, y_values_size = [], []

plt.rcParams['figure.raise_window'] = False

# Create the figure with five subplots for the timestamp, transaction count, gas used, TPS, and blocktime
fig, ((ax_ts, ax_tx), (ax_gas, ax_tps), (ax_bt, ax_size)) = plt.subplots(nrows=3, ncols=2, figsize=(15, 10))

ax_ts.set_xlabel("Block Number")
ax_ts.set_ylabel("Timestamp")
ax_tx.set_xlabel("Block Number")
ax_tx.set_ylabel("Transaction Count")
ax_gas.set_xlabel("Block Number")
ax_gas.set_ylabel("Gas Used")
ax_tps.set_xlabel("Block Number")
ax_tps.set_ylabel("TPS (transaction/sec)")
ax_bt.set_xlabel("Block Number")
ax_bt.set_ylabel("Blocktime (sec)")
ax_size.set_xlabel("Block Number")
ax_size.set_ylabel("Block Size (KB)")

# Define the update function for the timestamp subplot
def update_plot_ts(frame):
    global x_values_ts, y_values_ts
    cur = conn.cursor()
    cur.execute(f"SELECT number, \"timestamp\" FROM blocks ORDER BY number DESC LIMIT {fifo_size}")
    rows = cur.fetchall()[::-1]
    cur.close()
    # Append each row to the X and Y data points
    for row in rows:
        x_values_ts.append(row[0])
        y_values_ts.append(row[1])
    # Remove the oldest data points if the FIFO size is exceeded
    if len(x_values_ts) > fifo_size:
        x_values_ts = x_values_ts[-fifo_size:]
        y_values_ts = y_values_ts[-fifo_size:]
    ax_ts.clear()
    ax_ts.plot(x_values_ts, y_values_ts, linewidth=0.5, label="Timestamp")
    ax_ts.legend()
    plt.pause(0.01)

# Define the update function for the transaction count subplot
def update_plot_tx(frame):
    global x_values_tx, y_values_tx
    cur = conn.cursor()
    cur.execute(f"SELECT number, \"transactionsCount\" FROM blocks ORDER BY number DESC LIMIT {fifo_size}")
    rows = cur.fetchall()[::-1]
    cur.close()
    # Append each row to the X and Y data points
    for row in rows:
        x_values_tx.append(row[0])
        y_values_tx.append(row[1])
    # Remove the oldest data points if the FIFO size is exceeded
    if len(x_values_tx) > fifo_size:
        x_values_tx = x_values_tx[-fifo_size:]
        y_values_tx = y_values_tx[-fifo_size:]
    ax_tx.clear()
    ax_tx.plot(x_values_tx, y_values_tx, linewidth=0.5, label="Transaction Count")
    ax_tx.legend()
    plt.pause(0.01)

# Define the update function for the gas used subplot
def update_plot_gas(frame):
    global x_values_gas, y_values_gas
    cur = conn.cursor()
    cur.execute(f"SELECT number, \"gasUsed\" FROM blocks ORDER BY number DESC LIMIT {fifo_size}")
    rows = cur.fetchall()[::-1]
    cur.close()
    # Append each row to the X and Y data points
    for row in rows:
        x_values_gas.append(row[0])
        y_values_gas.append(row[1])
    # Remove the oldest data points if the FIFO size is exceeded
    if len(x_values_gas) > fifo_size:
        x_values_gas = x_values_gas[-fifo_size:]
        y_values_gas = y_values_gas[-fifo_size:]
    ax_gas.clear()
    ax_gas.plot(x_values_gas, y_values_gas, linewidth=0.5, label="Gas Used")
    ax_gas.legend()
    plt.pause(0.01)

# Define the update function for the TPS subplot
def update_plot_tps(frame):
    global x_values_tps, y_values_tps
    cur = conn.cursor()
    cur.execute(f"SELECT number, \"transactionsCount\", \"timestamp\" FROM blocks ORDER BY number DESC LIMIT {fifo_size}")
    rows = cur.fetchall()[::-1]
    cur.close()
    # Calculate TPS for each block except the last one and append to the X and Y data points
    tx_count_diff = 0
    for i in range(len(rows)-1):
        # tx_count_diff = rows[i+1][1] - rows[i][1]
        # if tx_count_diff > 0:
        #     time_diff = (datetime.fromtimestamp(rows[i+1][2]) - datetime.fromtimestamp(rows[i][2])).total_seconds()
        #     tps = tx_count_diff / time_diff if time_diff > 0 else 0
        # else:
        #     tps = 0
        time_diff = time_diff = (datetime.fromtimestamp(rows[i+1][2]) - datetime.fromtimestamp(rows[i][2])).total_seconds()
        tps = rows[i][1] / time_diff #if time_diff > 0 else 0


        x_values_tps.append(rows[i][0])
        y_values_tps.append(tps)
    # Add the last block separately with a TPS of 0
    x_values_tps.append(rows[-1][0])
    y_values_tps.append(y_values_tps[-1])
    # Remove the oldest data points if the FIFO size is exceeded
    if len(x_values_tps) > fifo_size:
        x_values_tps = x_values_tps[-fifo_size:]
        y_values_tps = y_values_tps[-fifo_size:]
    ax_tps.clear()
    ax_tps.plot(x_values_tps, y_values_tps, linewidth=0.5, label="TPS (transaction/sec)")
    ax_tps.legend()
    plt.pause(0.01)

# Define the update function for the blocktime subplot
def update_plot_bt(frame):
    global x_values_bt, y_values_bt
    cur = conn.cursor()
    cur.execute(f"SELECT number, \"timestamp\" FROM blocks ORDER BY number DESC LIMIT {fifo_size}")
    rows = cur.fetchall()[::-1]
    cur.close()
    # Calculate the blocktime difference for each block except the last one and append to the Y data points
    y_values_bt = []
    for i in range(len(rows)-1):
        time_diff = (datetime.fromtimestamp(rows[i+1][1]) - datetime.fromtimestamp(rows[i][1])).total_seconds()
        y_values_bt.append(time_diff)
        x_values_bt.append(rows[i][0])
    # Add the last block separately with a blocktime of same as the previous block
    x_values_bt.append(rows[-1][0])
    y_values_bt.append(y_values_bt[-1])
    # Remove the oldest data points if the FIFO size is exceeded
    if len(x_values_bt) > fifo_size:
        x_values_bt = x_values_bt[-fifo_size:]
        y_values_bt = y_values_bt[-fifo_size:]
    ax_bt.clear()
    ax_bt.plot(x_values_bt, y_values_bt, linewidth=0.5, label="Blocktime (sec)")
    ax_bt.legend()
    plt.pause(0.01)

def update_plot_size(frame):
    global x_values_size, y_values_size
    cur = conn.cursor()
    cur.execute(f"SELECT number, size FROM blocks ORDER BY number DESC LIMIT {fifo_size}")
    rows = cur.fetchall()[::-1]
    cur.close()
    # Append each row to the X and Y data points
    for row in rows:
        size_kb = row[1] / 1024
        x_values_size.append(row[0])
        y_values_size.append(size_kb)
    # Remove the oldest data points if the FIFO size is exceeded
    if len(x_values_size) > fifo_size:
        x_values_size = x_values_size[-fifo_size:]
        y_values_size = y_values_size[-fifo_size:]
    ax_size.clear()
    ax_size.plot(x_values_size, y_values_size, linewidth=0.5, label="Block Size (KB)")
    ax_size.legend()
    plt.pause(0.01)


# Create the initial plot for the timestamp, transaction count, gas used, TPS, and blocktime subplots
cur = conn.cursor()
cur.execute(f"SELECT number, \"timestamp\", \"transactionsCount\", \"gasUsed\", \"size\" FROM blocks ORDER BY number DESC LIMIT {page_size}")
rows = cur.fetchall()[::-1]
cur.close()
# Append each row to the X and Y data points for all five subplots
for row in rows:
    x_values_ts.append(row[0])
    y_values_ts.append(row[1])
    x_values_tx.append(row[0])
    y_values_tx.append(row[2])
    x_values_gas.append(row[0])
    y_values_gas.append(row[3])
    x_values_size.append(row[0])
    y_values_size.append(row[4])
ax_ts.plot(x_values_ts, y_values_ts, linewidth=0.5, label="Timestamp")
ax_tx.plot(x_values_tx, y_values_tx, linewidth=0.5, label="Transaction Count")
ax_gas.plot(x_values_gas, y_values_gas, linewidth=0.5, label="Gas Used")
ax_tps.plot(x_values_tps, y_values_tps, linewidth=0.5, label="TPS (transaction/sec)")
ax_bt.plot(x_values_bt, y_values_bt, linewidth=0.5, label="Blocktime (sec)")
ax_size.plot(x_values_size, y_values_size, linewidth=0.5, label="Block size (KB)")
ax_ts.legend()
ax_tx.legend()
ax_gas.legend()
ax_tps.legend()
ax_bt.legend()
ax_size.legend()

# Animate the subplots by updating them every few seconds
ani_ts = FuncAnimation(fig, update_plot_ts, interval=UPDATE_INTERVAL, blit=True, repeat=False)
ani_tx = FuncAnimation(fig, update_plot_tx, interval=UPDATE_INTERVAL, blit=True, repeat=False)
ani_gas = FuncAnimation(fig, update_plot_gas, interval=UPDATE_INTERVAL, blit=True, repeat=False)
ani_tps = FuncAnimation(fig, update_plot_tps, interval=UPDATE_INTERVAL, blit=True, repeat=False)
ani_bt = FuncAnimation(fig, update_plot_bt, interval=UPDATE_INTERVAL, blit=True, repeat=False)
ani_size = FuncAnimation(fig, update_plot_size, interval=UPDATE_INTERVAL, blit=True, repeat=False)

# Show the plot
try:
    plt.show()
except AttributeError:
    # handle the exception
    pass