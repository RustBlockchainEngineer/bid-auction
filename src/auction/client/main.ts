/**
 * test auction program
 */

  async function main() {

    console.log("tested successfully!");
  }
  
  main().then(
    () => process.exit(),
    err => {
      console.error(err);
      process.exit(-1);
    },
  );
  