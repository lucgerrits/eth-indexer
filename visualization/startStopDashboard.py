# Display data from START_BLOCK to STOP_BLOCK of the blockchain
# uses .env file in the parent directory to access environment variables
import psycopg2
import matplotlib.pyplot as plt
from dotenv import load_dotenv
import os
from datetime import datetime

# Define the start block and the stop block
START_BLOCK = 0
STOP_BLOCK = -1 # set to -1 to use the latest block

# or define the start block and the number of blocks to display after the start block:
PLOT_TIME_WINDOW_MINUTES = 60
AVERAGE_BLOCK_TIME_SECONDS = 3
NUMBER_OF_BLOCKS = int(PLOT_TIME_WINDOW_MINUTES * 60 / AVERAGE_BLOCK_TIME_SECONDS)
# STOP_BLOCK = START_BLOCK + NUMBER_OF_BLOCKS # uncomment this line to use this method

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


if(STOP_BLOCK == -1):
    cur = conn.cursor()
    cur.execute(f"SELECT number FROM blocks ORDER BY number DESC LIMIT 1")
    rows = cur.fetchall()
    cur.close()
    STOP_BLOCK = rows[0][0]

if(STOP_BLOCK < START_BLOCK):
    print("STOP_BLOCK must be greater than START_BLOCK")
    exit(1)

# Set the page size
page_size = STOP_BLOCK - START_BLOCK + 1

# Initialize empty lists for the X and Y data points
x_values_tx, y_values_tx = [], []
x_values_gas, y_values_gas = [], []
x_values_ts, y_values_ts = [], []
x_values_tps, y_values_tps = [], []
x_values_bt, y_values_bt = [], []
x_values_size, y_values_size = [], []

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

# Define the function to calculate TPS
def calc_tps(tx_count_diff, time_diff):
    if tx_count_diff > 0:
        tps = tx_count_diff / time_diff if time_diff > 0 else 0
    else:
        tps = 0
    return tps

# Query the database for the blocks between the start and stop blocks
cur = conn.cursor()
cur.execute(f"SELECT number, \"timestamp\", \"transactionsCount\", \"gasUsed\", \"size\" FROM blocks WHERE number BETWEEN {START_BLOCK} AND {STOP_BLOCK} ORDER BY number ASC")
rows = cur.fetchall()
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
    y_values_size.append(row[4] / 1024)

tx_count_diff = 0
for i in range(len(rows)-1):
    # tx_count_diff = rows[i+1][2] - rows[i][2]
    # if tx_count_diff > 0:
    #     time_diff = (datetime.fromtimestamp(rows[i+1][1]) - datetime.fromtimestamp(rows[i][1])).total_seconds()
    #     tps = tx_count_diff / time_diff #if time_diff > 0 else 0

    #     print(tx_count_diff, time_diff, tps)
    # else:
    #     tps = 0
    time_diff = (datetime.fromtimestamp(rows[i+1][1]) - datetime.fromtimestamp(rows[i][1])).total_seconds()
    tps = rows[i][2] / time_diff #if time_diff > 0 else 0

    x_values_tps.append(rows[i][0])
    y_values_tps.append(tps)
x_values_tps.append(rows[-1][0])
y_values_tps.append(y_values_tps[-1])

for i in range(len(rows)-1):
    time_diff = (datetime.fromtimestamp(rows[i+1][1]) - datetime.fromtimestamp(rows[i][1])).total_seconds()
    y_values_bt.append(time_diff)
    x_values_bt.append(rows[i][0])
x_values_bt.append(rows[-1][0])
y_values_bt.append(y_values_bt[-1])

# Plot the timestamp subplot
ax_ts.plot(x_values_ts, y_values_ts, linewidth=0.5, label="Timestamp")
ax_ts.legend()

# Plot the transaction count subplot
ax_tx.set_xlabel("Block Number")
ax_tx.set_ylabel("Transaction Count")
ax_tx.plot(x_values_tx, y_values_tx, linewidth=0.5, label="Transaction Count")
ax_tx.legend()

# Plot the gas used subplot
ax_gas.set_xlabel("Block Number")
ax_gas.set_ylabel("Gas Used")
ax_gas.plot(x_values_gas, y_values_gas, linewidth=0.5, label="Gas Used")
ax_gas.legend()

# Plot the TPS subplot
ax_tps.set_xlabel("Block Number")
ax_tps.set_ylabel("TPS (transaction/sec)")
ax_tps.plot(x_values_tps, y_values_tps, linewidth=0.5, label="TPS (transaction/sec)")
ax_tps.legend()

# Plot the blocktime subplot
ax_bt.set_xlabel("Block Number")
ax_bt.set_ylabel("Blocktime (sec)")
ax_bt.plot(x_values_bt, y_values_bt, linewidth=0.5, label="Blocktime (sec)")
ax_bt.legend()

# Plot the block size subplot
ax_size.set_xlabel("Block Number")
ax_size.set_ylabel("Block Size (KB)")
ax_size.plot(x_values_size, y_values_size, linewidth=0.5, label="Block Size (KB)")
ax_size.legend()

# plt.savefig("block_stats.png")

plt.show()
