import { useState } from "react";
import {
  Input,
  NumberInput,
  DateInput,
  Button,
  Modal,
  ModalContent,
  ModalHeader,
  ModalBody,
  ModalFooter,
} from "@heroui/react";

import {
    CalendarDate,
    parseDate,
} from '@internationalized/date';

interface SettingsModalProps {
  isOpen: boolean;
  onClose: (productCode: string | null, shipQuantity: number | null, shipDate: CalendarDate | null) => void;
}

function ShipByProductCodeModal({ isOpen, onClose }: SettingsModalProps) {
  const [productCode, setProductCode] = useState<string>("");
  const [shipQuantity, setShipQuantity] = useState<number>(0);
  const [shipDate, setShipDate] = useState<CalendarDate | null>(null);

  const renderDialogBody = () => (
    <div style={{ padding: "16px 0" }}>
      <div style={{ marginBottom: "24px" }}>
        <h3
          style={{
            fontSize: "16px",
            fontWeight: 600,
            marginBottom: "16px",
            color: "#333",
          }}
        >
          品番で出庫
        </h3>

        <div style={{ marginBottom: "20px" }}>
          <label
            style={{
              display: "block",
              marginBottom: "8px",
              fontWeight: 500,
              color: "#333",
            }}
          >
            品番
          </label>
          <Input value={productCode} onValueChange={setProductCode} />
          <label
            style={{
              display: "block",
              marginBottom: "8px",
              fontWeight: 500,
              color: "#333",
            }}
          >
            数量
          </label>
          <NumberInput value={shipQuantity} onValueChange={setShipQuantity} />
          <label
            style={{
              display: "block",
              marginBottom: "8px",
              fontWeight: 500,
              color: "#333",
            }}
          >
            出荷日
          </label>
          <DateInput
            value={shipDate}
            onChange={setShipDate}
            errorMessage={(value) => {
              if (value.isInvalid) {
                return "日付を入力してください。";
              }
            }}
          />
        </div>
      </div>
    </div>
  );

  return (
    <Modal
      isOpen={isOpen}
      onClose={() => onClose(null, null, null)}
      size="4xl"
      scrollBehavior="inside"
      classNames={{
        base: "max-h-[80vh]",
        body: "py-6",
      }}
    >
      <ModalContent>
        {() => (
          <>
            <ModalHeader className="flex flex-col gap-1">
              <h2 className="text-xl font-semibold">
                出庫(数量)
              </h2>
            </ModalHeader>
            <ModalBody>
                <div className="flex-1 min-w-0">{renderDialogBody()}</div>
            </ModalBody>
            <ModalFooter>
              <Button color="primary" onPress={() => onClose(productCode, shipQuantity, shipDate)}>
                出庫
              </Button>
            </ModalFooter>
          </>
        )}
      </ModalContent>
    </Modal>
  );
}

export default ShipByProductCodeModal;
